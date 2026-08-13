use crate::graph::{FactProvenance, GraphCitation, GraphNodeKind};
use crate::ingestion::LoadedProjectMemory;
use crate::retrieval::{ContextItem, ContextItemKind};
use crate::{LeyCoreError, AGENT_MEMORY_DIRECTORY};
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use directories::BaseDirs;
use model2vec_rs::model::StaticModel;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

pub const SEMANTIC_INDEX_SCHEMA_VERSION: u32 = 1;
pub const SEMANTIC_MODEL_ID: &str = "minishlab/potion-retrieval-32M";
pub const SEMANTIC_MODEL_REVISION: &str = "6fc8051fab2a1e0ee76689cf08c853792ac285e7";
pub const SEMANTIC_MODEL_DIMENSION: usize = 512;
pub const MAX_SEMANTIC_INDEX_ENTRIES: usize = 4_096;
pub const MAX_SEMANTIC_ENTRY_CHARACTERS: usize = 4_096;

const SEMANTIC_CACHE_APPLICATION_DIRECTORY: &str = "ley";
const SEMANTIC_CACHE_MODELS_DIRECTORY: &str = "models";
const SEMANTIC_CACHE_MODEL_DIRECTORY: &str = "minishlab--potion-retrieval-32m";
const SEMANTIC_INDEX_DIRECTORY: &str = "semantic-index";
const SEMANTIC_INDEX_FILE_PREFIX: &str = "semantic-index-v1-";
const SEMANTIC_INDEX_LIMIT_BYTES: u64 = 33_554_432;
const SEMANTIC_RRF_K: u32 = 60;
const MODEL_COPY_BUFFER_BYTES: usize = 64 * 1024;

static LOADED_SEMANTIC_MODEL: OnceLock<Mutex<Option<Arc<StaticModel>>>> = OnceLock::new();

const MODEL_FILES: [SemanticModelFile; 3] = [
    SemanticModelFile {
        name: "config.json",
        bytes: 202,
        sha256: "63c00d90824c832c04ec1d02b6a983fb90489bf049f29fbff15ba481b8a432ee",
    },
    SemanticModelFile {
        name: "tokenizer.json",
        bytes: 1_493_150,
        sha256: "7d75cbc54318138807c401b0f0c9721117c628b39de8e8e0edb6cb17e0ee7d18",
    },
    SemanticModelFile {
        name: "model.safetensors",
        bytes: 129_210_456,
        sha256: "07609e5bd33aad37900b3fd62f4ec96f6daec88ca4d46b9d8b928bfababf6ea0",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticModelFile {
    pub name: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticModelDescriptor {
    pub model_id: String,
    pub revision: String,
    pub dimension: usize,
    pub files: Vec<SemanticModelFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum SemanticModelStatus {
    Uninstalled { reason: String },
    Ready { model: SemanticModelDescriptor },
    Corrupt { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticModelInstallation {
    pub model: SemanticModelDescriptor,
    pub installed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SemanticIndexBinding {
    pub schema_version: u32,
    pub project_id: String,
    pub artifact_snapshot_id: String,
    pub graph_snapshot_id: String,
    pub model_id: String,
    pub model_revision: String,
    pub embedding_dimension: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticIndexState {
    Reused,
    Rebuilt,
    Unavailable,
}

/// A bounded text candidate for local, in-process semantic reranking.
///
/// Callers retain ownership of the source text. This helper never creates a semantic index or
/// contacts a remote service.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SemanticTextCandidate<'a> {
    pub id: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticTextRank {
    pub id: String,
    pub rank: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticTextRankOutcome {
    Available { ranks: Vec<SemanticTextRank> },
    Unavailable { reason: String },
}

/// The maximum number of already-bounded texts accepted by [`rank_bounded_local_texts`].
pub(crate) const MAX_SEMANTIC_RANK_TEXTS: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SemanticSearchOutcome {
    Available {
        items: Vec<ContextItem>,
        index_state: SemanticIndexState,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SemanticIndexManifest {
    binding: SemanticIndexBinding,
    entries: Vec<SemanticIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SemanticIndexEntry {
    id: String,
    kind: ContextItemKind,
    title: String,
    path: Option<String>,
    language: Option<String>,
    citation: GraphCitation,
    provenance: FactProvenance,
    confidence: f32,
    embedding: Vec<f32>,
}

/// Returns the only reviewed semantic-retrieval model Ley supports.
pub fn supported_semantic_model() -> SemanticModelDescriptor {
    SemanticModelDescriptor {
        model_id: SEMANTIC_MODEL_ID.to_owned(),
        revision: SEMANTIC_MODEL_REVISION.to_owned(),
        dimension: SEMANTIC_MODEL_DIMENSION,
        files: MODEL_FILES.to_vec(),
    }
}

/// Returns Ley's private per-user cache location for the pinned model.
///
/// This does not create the directory and never accesses the network.
pub fn default_semantic_model_cache_path() -> Result<PathBuf, LeyCoreError> {
    let base = BaseDirs::new().ok_or(LeyCoreError::ConfigDirectoryUnavailable)?;
    Ok(base
        .cache_dir()
        .join(SEMANTIC_CACHE_APPLICATION_DIRECTORY)
        .join(SEMANTIC_CACHE_MODELS_DIRECTORY)
        .join(SEMANTIC_CACHE_MODEL_DIRECTORY)
        .join(SEMANTIC_MODEL_REVISION))
}

/// Inspects the default model cache without resolving or downloading anything.
pub fn semantic_model_status() -> SemanticModelStatus {
    match default_semantic_model_cache_path() {
        Ok(path) => semantic_model_status_at(&path),
        Err(_) => SemanticModelStatus::Uninstalled {
            reason: "no operating-system cache directory is available".to_owned(),
        },
    }
}

/// Inspects an explicit local model directory without resolving or downloading anything.
pub fn semantic_model_status_at(model_directory: impl AsRef<Path>) -> SemanticModelStatus {
    match verify_model_directory(model_directory.as_ref()) {
        Ok(()) => SemanticModelStatus::Ready {
            model: supported_semantic_model(),
        },
        Err(ModelDirectoryError::Missing) => SemanticModelStatus::Uninstalled {
            reason: "the pinned local semantic model is not installed".to_owned(),
        },
        Err(ModelDirectoryError::Corrupt(reason)) => SemanticModelStatus::Corrupt { reason },
    }
}

/// Verifies a locally staged model and atomically installs it into Ley's default cache.
///
/// The staging directory is read locally only. Files are copied with a fixed-size buffer so
/// callers can download the 129 MB payload independently without retaining it in memory.
pub fn install_semantic_model_from_staging(
    staging_directory: impl AsRef<Path>,
) -> Result<SemanticModelInstallation, LeyCoreError> {
    let destination = default_semantic_model_cache_path()?;
    install_semantic_model_from_staging_at(staging_directory, destination)
}

/// Verifies a locally staged model and atomically installs it into an explicit cache directory.
/// This is primarily useful to first-party callers that manage an OS-specific cache root.
pub fn install_semantic_model_from_staging_at(
    staging_directory: impl AsRef<Path>,
    destination_directory: impl AsRef<Path>,
) -> Result<SemanticModelInstallation, LeyCoreError> {
    let staging_directory = staging_directory.as_ref();
    let destination_directory = destination_directory.as_ref();
    match verify_model_directory(staging_directory) {
        Ok(()) => {}
        Err(ModelDirectoryError::Missing) => {
            return Err(LeyCoreError::SemanticModelInstallation(
                "the local staging directory is missing required model files".to_owned(),
            ));
        }
        Err(ModelDirectoryError::Corrupt(reason)) => {
            return Err(LeyCoreError::SemanticModelInstallation(format!(
                "the local staging directory failed verification: {reason}"
            )));
        }
    }

    let replacing_invalid_install = if destination_directory.exists() {
        match semantic_model_status_at(destination_directory) {
            SemanticModelStatus::Ready { model } => {
                return Ok(SemanticModelInstallation {
                    model,
                    installed: false,
                });
            }
            SemanticModelStatus::Uninstalled { .. } | SemanticModelStatus::Corrupt { .. } => true,
        }
    } else {
        false
    };

    let parent = destination_directory.parent().ok_or_else(|| {
        LeyCoreError::SemanticModelInstallation(
            "the local model cache destination has no parent directory".to_owned(),
        )
    })?;
    ensure_private_directory_tree(parent)?;
    let staging_copy = unique_install_staging_directory(parent)?;
    let result = (|| {
        copy_verified_model_directory(staging_directory, &staging_copy)?;
        let backup = if replacing_invalid_install {
            let backup = unique_install_backup_path(parent)?;
            fs::rename(destination_directory, &backup).map_err(|_| {
                LeyCoreError::SemanticModelInstallation(
                    "could not isolate the invalid local model before repair".to_owned(),
                )
            })?;
            Some(backup)
        } else {
            None
        };
        if let Err(source) = fs::rename(&staging_copy, destination_directory) {
            if let Some(backup) = &backup {
                let _ = fs::rename(backup, destination_directory);
            }
            if source.kind() == std::io::ErrorKind::AlreadyExists
                && matches!(
                    semantic_model_status_at(destination_directory),
                    SemanticModelStatus::Ready { .. }
                )
            {
                return Ok(SemanticModelInstallation {
                    model: supported_semantic_model(),
                    installed: false,
                });
            }
            return Err(LeyCoreError::SemanticModelInstallation(
                "could not atomically promote the verified local model into the cache".to_owned(),
            ));
        }
        let installation = match semantic_model_status_at(destination_directory) {
            SemanticModelStatus::Ready { model } => SemanticModelInstallation {
                model,
                installed: true,
            },
            SemanticModelStatus::Uninstalled { .. } | SemanticModelStatus::Corrupt { .. } => {
                let _ = remove_local_path(destination_directory);
                if let Some(backup) = &backup {
                    let _ = fs::rename(backup, destination_directory);
                }
                return Err(LeyCoreError::SemanticModelInstallation(
                    "the installed local semantic model failed post-install verification"
                        .to_owned(),
                ));
            }
        };
        if let Some(backup) = &backup {
            remove_local_path(backup).map_err(|_| {
                LeyCoreError::SemanticModelInstallation(
                    "the repaired model is ready but the invalid backup could not be removed"
                        .to_owned(),
                )
            })?;
        }
        Ok(installation)
    })();
    if result.is_err() {
        let _ = remove_local_path(&staging_copy);
    }
    result
}

/// Ranks a caller-owned, already-selected set of texts with Ley's verified local model.
///
/// The candidate count and each candidate document are bounded before embedding. A missing,
/// corrupt, unavailable, or unexpectedly-behaving local model becomes an explicit unavailable
/// outcome so callers can retain lexical-only results without exposing local paths.
pub(crate) fn rank_bounded_local_texts(
    query: &str,
    candidates: &[SemanticTextCandidate<'_>],
) -> SemanticTextRankOutcome {
    if candidates.len() > MAX_SEMANTIC_RANK_TEXTS {
        return SemanticTextRankOutcome::Unavailable {
            reason: format!(
                "the local semantic rank request exceeded its {}-candidate limit",
                MAX_SEMANTIC_RANK_TEXTS
            ),
        };
    }
    let mut identifiers = BTreeSet::new();
    if candidates.iter().any(|candidate| {
        candidate.id.is_empty() || candidate.text.is_empty() || !identifiers.insert(candidate.id)
    }) {
        return SemanticTextRankOutcome::Unavailable {
            reason: "the local semantic rank request has invalid candidate identifiers or text"
                .to_owned(),
        };
    }
    if candidates.is_empty() {
        return SemanticTextRankOutcome::Available { ranks: Vec::new() };
    }

    let model_directory =
        match default_semantic_model_cache_path() {
            Ok(path) => path,
            Err(_) => return SemanticTextRankOutcome::Unavailable {
                reason:
                    "no operating-system cache directory is available for the local semantic model"
                        .to_owned(),
            },
        };
    let model = match loaded_semantic_model(&model_directory) {
        Ok(model) => model,
        Err(reason) => return SemanticTextRankOutcome::Unavailable { reason },
    };
    let query_embedding = model.encode_single(query);
    if !valid_embedding(&query_embedding) {
        return SemanticTextRankOutcome::Unavailable {
            reason: "the local semantic model returned an unexpected query embedding".to_owned(),
        };
    }
    let texts = candidates
        .iter()
        .map(|candidate| bounded_document(candidate.text))
        .collect::<Vec<_>>();
    let embeddings = model.encode_with_args(&texts, Some(512), 32);
    if embeddings.len() != candidates.len()
        || embeddings
            .iter()
            .any(|embedding| !valid_embedding(embedding))
    {
        return SemanticTextRankOutcome::Unavailable {
            reason: "the local semantic model returned invalid candidate embeddings".to_owned(),
        };
    }
    let mut ranks = candidates
        .iter()
        .zip(embeddings)
        .map(|(candidate, embedding)| {
            (
                cosine_similarity(&query_embedding, &embedding),
                candidate.id,
            )
        })
        .collect::<Vec<_>>();
    ranks.sort_by(|(left_score, left_id), (right_score, right_id)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_id.cmp(right_id))
    });
    SemanticTextRankOutcome::Available {
        ranks: ranks
            .into_iter()
            .enumerate()
            .map(|(index, (_, id))| SemanticTextRank {
                id: id.to_owned(),
                rank: index as u32 + 1,
            })
            .collect(),
    }
}

pub(crate) fn semantic_ranked_project_context(
    memory: &LoadedProjectMemory,
    vault: &Path,
    query: &str,
) -> SemanticSearchOutcome {
    let model_directory =
        match default_semantic_model_cache_path() {
            Ok(path) => path,
            Err(_) => return SemanticSearchOutcome::Unavailable {
                reason:
                    "no operating-system cache directory is available for the local semantic model"
                        .to_owned(),
            },
        };
    let model = match loaded_semantic_model(&model_directory) {
        Ok(model) => model,
        Err(reason) => return SemanticSearchOutcome::Unavailable { reason },
    };

    let binding = semantic_index_binding(memory);
    let (index, index_state) = match load_or_build_semantic_index(memory, vault, &binding, &model) {
        Ok(result) => result,
        Err(_) => {
            return SemanticSearchOutcome::Unavailable {
                reason: "the snapshot-bound semantic index could not be built".to_owned(),
            }
        }
    };
    let query_embedding = model.encode_single(query);
    if !valid_embedding(&query_embedding) {
        return SemanticSearchOutcome::Unavailable {
            reason: "the local semantic model returned an unexpected embedding dimension"
                .to_owned(),
        };
    }

    let mut ranked = index
        .entries
        .iter()
        .map(|entry| (cosine_similarity(&query_embedding, &entry.embedding), entry))
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left.id.cmp(&right.id))
    });
    SemanticSearchOutcome::Available {
        items: ranked
            .into_iter()
            .map(|(_, entry)| context_item_from_semantic_entry(entry))
            .collect(),
        index_state,
    }
}

fn loaded_semantic_model(model_directory: &Path) -> Result<Arc<StaticModel>, String> {
    let cache = LOADED_SEMANTIC_MODEL.get_or_init(|| Mutex::new(None));
    let mut loaded = cache
        .lock()
        .map_err(|_| "the local semantic model cache is unavailable".to_owned())?;
    if let Some(model) = loaded.as_ref() {
        return Ok(Arc::clone(model));
    }
    match semantic_model_status_at(model_directory) {
        SemanticModelStatus::Ready { .. } => {}
        SemanticModelStatus::Uninstalled { reason } | SemanticModelStatus::Corrupt { reason } => {
            return Err(reason);
        }
    }
    // The crate is compiled with `local-only`, so even an invalid path cannot trigger a download.
    let model = StaticModel::from_pretrained(model_directory, None, Some(true), None)
        .map_err(|_| "the verified local semantic model could not be loaded".to_owned())?;
    let model = Arc::new(model);
    *loaded = Some(Arc::clone(&model));
    Ok(model)
}

pub(crate) fn reciprocal_rank_fusion(
    lexical: Vec<ContextItem>,
    semantic: Vec<ContextItem>,
) -> Vec<ContextItem> {
    let mut merged = BTreeMap::<String, FusedContextItem>::new();
    for (index, item) in lexical.into_iter().enumerate() {
        let rank = index as u32 + 1;
        let entry = merged
            .entry(item.id.clone())
            .or_insert_with(|| FusedContextItem { item, score: 0.0 });
        entry.score += 1.0 / f64::from(SEMANTIC_RRF_K + rank);
    }
    for (index, item) in semantic.into_iter().enumerate() {
        let rank = index as u32 + 1;
        let entry = merged
            .entry(item.id.clone())
            .or_insert_with(|| FusedContextItem { item, score: 0.0 });
        entry.score += 1.0 / f64::from(SEMANTIC_RRF_K + rank);
    }
    let mut results = merged.into_values().collect::<Vec<_>>();
    results.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.item.id.cmp(&right.item.id))
    });
    results.into_iter().map(|entry| entry.item).collect()
}

#[derive(Debug, Clone)]
struct FusedContextItem {
    item: ContextItem,
    score: f64,
}

fn semantic_index_binding(memory: &LoadedProjectMemory) -> SemanticIndexBinding {
    SemanticIndexBinding {
        schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
        project_id: memory.manifest.project_id.clone(),
        artifact_snapshot_id: memory.manifest.snapshot_id.clone(),
        graph_snapshot_id: memory.graph.graph_snapshot_id.clone(),
        model_id: SEMANTIC_MODEL_ID.to_owned(),
        model_revision: SEMANTIC_MODEL_REVISION.to_owned(),
        embedding_dimension: SEMANTIC_MODEL_DIMENSION,
    }
}

fn load_or_build_semantic_index(
    memory: &LoadedProjectMemory,
    vault: &Path,
    binding: &SemanticIndexBinding,
    model: &StaticModel,
) -> Result<(SemanticIndexManifest, SemanticIndexState), LeyCoreError> {
    let directory = open_semantic_index_directory(vault, &binding.project_id)?;
    let filename = semantic_index_filename(binding)?;
    if let Some(bytes) =
        read_optional_private_file(&directory, &filename, SEMANTIC_INDEX_LIMIT_BYTES)?
    {
        if let Ok(index) = serde_json::from_slice::<SemanticIndexManifest>(&bytes) {
            if validate_semantic_index(&index, binding).is_ok() {
                return Ok((index, SemanticIndexState::Reused));
            }
        }
    }

    let index = build_semantic_index(memory, binding, model)?;
    let body = serde_json::to_vec(&index)
        .map_err(|error| LeyCoreError::InvalidSemanticIndex(error.to_string()))?;
    if body.len() as u64 > SEMANTIC_INDEX_LIMIT_BYTES {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "the bounded semantic index exceeded its storage limit".to_owned(),
        ));
    }
    write_atomic_private(&directory, &filename, &body)?;
    Ok((index, SemanticIndexState::Rebuilt))
}

fn build_semantic_index(
    memory: &LoadedProjectMemory,
    binding: &SemanticIndexBinding,
    model: &StaticModel,
) -> Result<SemanticIndexManifest, LeyCoreError> {
    let candidates = semantic_index_candidates(memory)?;
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.clone())
        .collect::<Vec<_>>();
    let embeddings = model.encode_with_args(&texts, Some(512), 32);
    if embeddings.len() != candidates.len()
        || embeddings
            .iter()
            .any(|embedding| !valid_embedding(embedding))
    {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "local semantic model returned invalid embeddings".to_owned(),
        ));
    }
    let entries = candidates
        .into_iter()
        .zip(embeddings)
        .map(|(candidate, embedding)| SemanticIndexEntry {
            id: candidate.item.id,
            kind: candidate.item.kind,
            title: candidate.item.title,
            path: candidate.item.path,
            language: candidate.item.language,
            citation: candidate.item.citation,
            provenance: candidate.item.provenance,
            confidence: candidate.item.confidence,
            embedding,
        })
        .collect::<Vec<_>>();
    let index = SemanticIndexManifest {
        binding: binding.clone(),
        entries,
    };
    validate_semantic_index(&index, binding)?;
    Ok(index)
}

#[derive(Debug)]
struct SemanticIndexCandidate {
    item: ContextItem,
    text: String,
}

fn semantic_index_candidates(
    memory: &LoadedProjectMemory,
) -> Result<Vec<SemanticIndexCandidate>, LeyCoreError> {
    let mut candidates = Vec::new();
    for artifact in &memory.manifest.files {
        if candidates.len() >= MAX_SEMANTIC_INDEX_ENTRIES {
            break;
        }
        let Some(text) = memory.read_artifact_text(artifact)? else {
            continue;
        };
        let document = bounded_document(&format!("{}\n{}", artifact.path, text));
        candidates.push(SemanticIndexCandidate {
            item: ContextItem {
                id: format!("artifact:{}", artifact.path),
                kind: ContextItemKind::Artifact,
                title: artifact.path.clone(),
                path: Some(artifact.path.clone()),
                language: artifact.language.clone(),
                snippet: None,
                citation: GraphCitation {
                    artifact_path: artifact.path.clone(),
                    start_line: 1,
                    start_column: 1,
                    end_line: artifact.line_count.max(1),
                    end_column: 1,
                    content_hash: artifact.content_hash.clone(),
                    artifact_snapshot_id: memory.manifest.snapshot_id.clone(),
                },
                score: 0,
                provenance: FactProvenance::Deterministic,
                confidence: 1.0,
                trust_state: "direct-evidence",
                source_boundary: "untrusted-project-evidence",
            },
            text: document,
        });
    }
    for node in &memory.graph.nodes {
        if candidates.len() >= MAX_SEMANTIC_INDEX_ENTRIES {
            break;
        }
        let kind = match node.kind {
            GraphNodeKind::Symbol => ContextItemKind::Symbol,
            GraphNodeKind::Dependency => ContextItemKind::Dependency,
            _ => continue,
        };
        let Some(citation) = node.citation.clone() else {
            continue;
        };
        let mut document = node.name.clone();
        if let Some(path) = &node.path {
            document.push(' ');
            document.push_str(path);
        }
        if let Some(symbol_kind) = &node.symbol_kind {
            document.push(' ');
            document.push_str(symbol_kind);
        }
        if let Some(package_manager) = &node.package_manager {
            document.push(' ');
            document.push_str(package_manager);
        }
        candidates.push(SemanticIndexCandidate {
            item: ContextItem {
                id: node.id.clone(),
                kind,
                title: node.name.clone(),
                path: node.path.clone(),
                language: node.language.clone(),
                snippet: None,
                citation,
                score: 0,
                provenance: node.provenance,
                confidence: node.confidence,
                trust_state: "direct-evidence",
                source_boundary: "untrusted-project-evidence",
            },
            text: bounded_document(&document),
        });
    }
    candidates.sort_by(|left, right| left.item.id.cmp(&right.item.id));
    candidates.dedup_by(|left, right| left.item.id == right.item.id);
    if candidates.is_empty() {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "the bound snapshot has no retained semantic candidates".to_owned(),
        ));
    }
    Ok(candidates)
}

fn bounded_document(value: &str) -> String {
    if value.chars().count() <= MAX_SEMANTIC_ENTRY_CHARACTERS {
        value.to_owned()
    } else {
        value.chars().take(MAX_SEMANTIC_ENTRY_CHARACTERS).collect()
    }
}

fn context_item_from_semantic_entry(entry: &SemanticIndexEntry) -> ContextItem {
    ContextItem {
        id: entry.id.clone(),
        kind: entry.kind,
        title: entry.title.clone(),
        path: entry.path.clone(),
        language: entry.language.clone(),
        snippet: None,
        citation: entry.citation.clone(),
        score: 0,
        provenance: entry.provenance,
        confidence: entry.confidence,
        trust_state: "direct-evidence",
        source_boundary: "untrusted-project-evidence",
    }
}

fn validate_semantic_index(
    index: &SemanticIndexManifest,
    expected_binding: &SemanticIndexBinding,
) -> Result<(), LeyCoreError> {
    if &index.binding != expected_binding {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "semantic index binding does not match the selected snapshot and model".to_owned(),
        ));
    }
    if index.entries.is_empty() || index.entries.len() > MAX_SEMANTIC_INDEX_ENTRIES {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "semantic index entry count is outside the supported bound".to_owned(),
        ));
    }
    let mut previous_id: Option<&str> = None;
    let mut ids = BTreeSet::new();
    for entry in &index.entries {
        if entry.id.is_empty()
            || entry.title.is_empty()
            || entry.citation.artifact_snapshot_id != expected_binding.artifact_snapshot_id
            || !valid_embedding(&entry.embedding)
            || !entry.confidence.is_finite()
        {
            return Err(LeyCoreError::InvalidSemanticIndex(
                "semantic index contains invalid entry data".to_owned(),
            ));
        }
        if previous_id.is_some_and(|previous| previous >= entry.id.as_str())
            || !ids.insert(entry.id.as_str())
        {
            return Err(LeyCoreError::InvalidSemanticIndex(
                "semantic index entries must have stable sorted identifiers".to_owned(),
            ));
        }
        previous_id = Some(&entry.id);
    }
    Ok(())
}

fn semantic_index_filename(binding: &SemanticIndexBinding) -> Result<String, LeyCoreError> {
    let bytes = serde_json::to_vec(binding)
        .map_err(|error| LeyCoreError::InvalidSemanticIndex(error.to_string()))?;
    Ok(format!(
        "{SEMANTIC_INDEX_FILE_PREFIX}{}.json",
        sha256_hex(&bytes)
    ))
}

fn open_semantic_index_directory(vault: &Path, project_id: &str) -> Result<Dir, LeyCoreError> {
    let canonical_vault = vault.canonicalize().map_err(|source| LeyCoreError::Io {
        path: vault.to_path_buf(),
        source,
    })?;
    if !canonical_vault.is_dir() {
        return Err(LeyCoreError::NotDirectory(vault.to_path_buf()));
    }
    let vault_dir =
        Dir::open_ambient_dir(&canonical_vault, ambient_authority()).map_err(|source| {
            LeyCoreError::Io {
                path: canonical_vault,
                source,
            }
        })?;
    let ley_dir = open_existing_dir(&vault_dir, ".ley")?;
    let memory_dir = open_existing_dir(&ley_dir, AGENT_MEMORY_DIRECTORY)?;
    let projects_dir = open_existing_dir(&memory_dir, "projects")?;
    let project_dir = open_existing_dir(&projects_dir, project_id)?;
    open_or_create_private_dir(&project_dir, SEMANTIC_INDEX_DIRECTORY)
}

fn open_existing_dir(parent: &Dir, name: &str) -> Result<Dir, LeyCoreError> {
    use cap_fs_ext::DirExt;
    parent.open_dir_nofollow(name).map_err(|source| {
        LeyCoreError::InvalidSemanticIndex(format!(
            "the bound project memory namespace is unavailable ({})",
            source.kind()
        ))
    })
}

fn open_or_create_private_dir(parent: &Dir, name: &str) -> Result<Dir, LeyCoreError> {
    use cap_fs_ext::DirExt;
    match parent.open_dir_nofollow(name) {
        Ok(directory) => {
            ensure_private_dir_permissions(&directory)?;
            return Ok(directory);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(LeyCoreError::InvalidSemanticIndex(format!(
                "could not open the derived semantic index directory ({})",
                source.kind()
            )));
        }
    }
    let mut builder = cap_std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use cap_std::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match parent.create_dir_with(name, &builder) {
        Ok(()) => {}
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(source) => {
            return Err(LeyCoreError::InvalidSemanticIndex(format!(
                "could not create the derived semantic index directory ({})",
                source.kind()
            )));
        }
    }
    let directory = parent.open_dir_nofollow(name).map_err(|source| {
        LeyCoreError::InvalidSemanticIndex(format!(
            "could not open the derived semantic index directory ({})",
            source.kind()
        ))
    })?;
    ensure_private_dir_permissions(&directory)?;
    Ok(directory)
}

fn read_optional_private_file(
    directory: &Dir,
    name: &str,
    limit: u64,
) -> Result<Option<Vec<u8>>, LeyCoreError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = match directory.open_with(name, &options) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(LeyCoreError::InvalidSemanticIndex(format!(
                "could not read a derived semantic index file ({})",
                source.kind()
            )));
        }
    };
    ensure_private_file_permissions(&file)?;
    let metadata = file.metadata().map_err(|source| {
        LeyCoreError::InvalidSemanticIndex(format!(
            "could not inspect a derived semantic index file ({})",
            source.kind()
        ))
    })?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "a derived semantic index file is not a bounded regular private file".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| {
            LeyCoreError::InvalidSemanticIndex(format!(
                "could not read a derived semantic index file ({})",
                source.kind()
            ))
        })?;
    if bytes.len() as u64 > limit {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "a derived semantic index file exceeds its size limit".to_owned(),
        ));
    }
    Ok(Some(bytes))
}

fn write_atomic_private(directory: &Dir, name: &str, body: &[u8]) -> Result<(), LeyCoreError> {
    let mut temporary = cap_tempfile::TempFile::new(directory).map_err(|source| {
        LeyCoreError::InvalidSemanticIndex(format!(
            "could not create a derived semantic index file ({})",
            source.kind()
        ))
    })?;
    let mut permissions = temporary
        .as_file()
        .metadata()
        .map_err(|source| LeyCoreError::InvalidSemanticIndex(source.kind().to_string()))?
        .permissions();
    permissions.set_readonly(false);
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        permissions.set_mode(0o600);
    }
    temporary
        .as_file()
        .set_permissions(permissions)
        .map_err(|source| LeyCoreError::InvalidSemanticIndex(source.kind().to_string()))?;
    temporary
        .write_all(body)
        .map_err(|source| LeyCoreError::InvalidSemanticIndex(source.kind().to_string()))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|source| LeyCoreError::InvalidSemanticIndex(source.kind().to_string()))?;
    temporary
        .replace(name)
        .map_err(|source| LeyCoreError::InvalidSemanticIndex(source.kind().to_string()))
}

fn ensure_private_dir_permissions(directory: &Dir) -> Result<(), LeyCoreError> {
    let metadata = directory.dir_metadata().map_err(|source| {
        LeyCoreError::InvalidSemanticIndex(format!(
            "could not inspect the derived semantic index directory ({})",
            source.kind()
        ))
    })?;
    if !metadata.is_dir() {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "the derived semantic index location is not a directory".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidSemanticIndex(
                "the derived semantic index directory must have private permissions".to_owned(),
            ));
        }
    }
    Ok(())
}

fn ensure_private_file_permissions(file: &cap_std::fs::File) -> Result<(), LeyCoreError> {
    let metadata = file.metadata().map_err(|source| {
        LeyCoreError::InvalidSemanticIndex(format!(
            "could not inspect a derived semantic index file ({})",
            source.kind()
        ))
    })?;
    if !metadata.is_file() {
        return Err(LeyCoreError::InvalidSemanticIndex(
            "a derived semantic index entry is not a regular file".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        use cap_std::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(LeyCoreError::InvalidSemanticIndex(
                "a derived semantic index file must have private permissions".to_owned(),
            ));
        }
    }
    Ok(())
}

fn valid_embedding(embedding: &[f32]) -> bool {
    embedding.len() == SEMANTIC_MODEL_DIMENSION && embedding.iter().all(|value| value.is_finite())
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    if !valid_embedding(left) || !valid_embedding(right) {
        return f64::NEG_INFINITY;
    }
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (left, right) in left.iter().zip(right) {
        let left = f64::from(*left);
        let right = f64::from(*right);
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt()).max(f64::MIN_POSITIVE)
}

enum ModelDirectoryError {
    Missing,
    Corrupt(String),
}

fn verify_model_directory(directory: &Path) -> Result<(), ModelDirectoryError> {
    let metadata = match fs::symlink_metadata(directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelDirectoryError::Missing)
        }
        Err(_) => {
            return Err(ModelDirectoryError::Corrupt(
                "the local model directory cannot be inspected".to_owned(),
            ))
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ModelDirectoryError::Corrupt(
            "the local model location is not a regular directory".to_owned(),
        ));
    }
    if !has_private_permissions(&metadata) {
        return Err(ModelDirectoryError::Corrupt(
            "the local model directory must have private permissions".to_owned(),
        ));
    }
    for required in MODEL_FILES {
        verify_model_file(directory, required)?;
    }
    Ok(())
}

fn verify_model_file(
    directory: &Path,
    required: SemanticModelFile,
) -> Result<(), ModelDirectoryError> {
    let path = directory.join(required.name);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ModelDirectoryError::Corrupt(format!(
                "{} is missing",
                required.name
            )))
        }
        Err(_) => {
            return Err(ModelDirectoryError::Corrupt(format!(
                "{} cannot be inspected",
                required.name
            )))
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ModelDirectoryError::Corrupt(format!(
            "{} is not a regular file",
            required.name
        )));
    }
    if !has_private_permissions(&metadata) {
        return Err(ModelDirectoryError::Corrupt(format!(
            "{} must have private permissions",
            required.name
        )));
    }
    if metadata.len() != required.bytes {
        return Err(ModelDirectoryError::Corrupt(format!(
            "{} has an unexpected size",
            required.name
        )));
    }
    let mut file = fs::File::open(&path)
        .map_err(|_| ModelDirectoryError::Corrupt(format!("{} cannot be read", required.name)))?;
    let digest = sha256_reader(&mut file)
        .map_err(|_| ModelDirectoryError::Corrupt(format!("{} cannot be read", required.name)))?;
    if digest != required.sha256 {
        return Err(ModelDirectoryError::Corrupt(format!(
            "{} failed checksum verification",
            required.name
        )));
    }
    Ok(())
}

fn ensure_private_directory_tree(directory: &Path) -> Result<(), LeyCoreError> {
    let mut missing = Vec::new();
    let mut cursor = directory;
    while !cursor.exists() {
        missing.push(cursor.to_path_buf());
        cursor = cursor.parent().ok_or_else(|| {
            LeyCoreError::SemanticModelInstallation(
                "the local model cache destination has no existing ancestor".to_owned(),
            )
        })?;
    }
    let metadata = fs::symlink_metadata(cursor).map_err(|_| {
        LeyCoreError::SemanticModelInstallation(
            "the local model cache parent cannot be inspected".to_owned(),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LeyCoreError::SemanticModelInstallation(
            "the local model cache parent is not a regular directory".to_owned(),
        ));
    }
    for path in missing.into_iter().rev() {
        fs::create_dir(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                LeyCoreError::SemanticModelInstallation(
                    "the local model cache directory changed during installation".to_owned(),
                )
            } else {
                LeyCoreError::SemanticModelInstallation(
                    "could not create a private local model cache directory".to_owned(),
                )
            }
        })?;
        set_private_directory_permissions(&path)?;
    }
    Ok(())
}

fn unique_install_staging_directory(parent: &Path) -> Result<PathBuf, LeyCoreError> {
    for attempt in 0..100_u32 {
        let name = format!(
            ".{}-{}-{}",
            SEMANTIC_CACHE_MODEL_DIRECTORY,
            std::process::id(),
            attempt
        );
        let path = parent.join(name);
        match fs::create_dir(&path) {
            Ok(()) => {
                set_private_directory_permissions(&path)?;
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                return Err(LeyCoreError::SemanticModelInstallation(
                    "could not create private local model staging".to_owned(),
                ))
            }
        }
    }
    Err(LeyCoreError::SemanticModelInstallation(
        "could not reserve private local model staging".to_owned(),
    ))
}

fn unique_install_backup_path(parent: &Path) -> Result<PathBuf, LeyCoreError> {
    for attempt in 0..100_u32 {
        let path = parent.join(format!(
            ".{}-invalid-{}-{}",
            SEMANTIC_CACHE_MODEL_DIRECTORY,
            std::process::id(),
            attempt
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(LeyCoreError::SemanticModelInstallation(
        "could not reserve a private repair backup path".to_owned(),
    ))
}

fn remove_local_path(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)
        }
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn copy_verified_model_directory(source: &Path, destination: &Path) -> Result<(), LeyCoreError> {
    for required in MODEL_FILES {
        let source_file = source.join(required.name);
        let destination_file = destination.join(required.name);
        let mut input = fs::File::open(&source_file).map_err(|_| {
            LeyCoreError::SemanticModelInstallation(
                "a verified staging file could not be opened".to_owned(),
            )
        })?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&destination_file).map_err(|_| {
            LeyCoreError::SemanticModelInstallation(
                "a private model cache file could not be created".to_owned(),
            )
        })?;
        let mut digest = Sha256::new();
        let mut copied = 0_u64;
        let mut buffer = [0_u8; MODEL_COPY_BUFFER_BYTES];
        loop {
            let read = input.read(&mut buffer).map_err(|_| {
                LeyCoreError::SemanticModelInstallation(
                    "a verified staging file could not be read".to_owned(),
                )
            })?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read]).map_err(|_| {
                LeyCoreError::SemanticModelInstallation(
                    "a private model cache file could not be written".to_owned(),
                )
            })?;
            digest.update(&buffer[..read]);
            copied = copied.saturating_add(read as u64);
        }
        output.sync_all().map_err(|_| {
            LeyCoreError::SemanticModelInstallation(
                "a private model cache file could not be synced".to_owned(),
            )
        })?;
        if copied != required.bytes || hex_digest(digest.finalize()) != required.sha256 {
            return Err(LeyCoreError::SemanticModelInstallation(
                "a staging file changed during installation".to_owned(),
            ));
        }
        set_private_file_permissions(&destination_file)?;
    }
    Ok(())
}

fn sha256_reader(reader: &mut fs::File) -> std::io::Result<String> {
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; MODEL_COPY_BUFFER_BYTES];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex_digest(digest.finalize()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn has_private_permissions(metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o077 == 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        true
    }
}

fn set_private_directory_permissions(path: &Path) -> Result<(), LeyCoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| {
            LeyCoreError::SemanticModelInstallation(
                "could not set private permissions on a local model cache directory".to_owned(),
            )
        })?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn set_private_file_permissions(path: &Path) -> Result<(), LeyCoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|_| {
            LeyCoreError::SemanticModelInstallation(
                "could not set private permissions on a local model cache file".to_owned(),
            )
        })?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphCitation;

    fn item(id: &str) -> ContextItem {
        ContextItem {
            id: id.to_owned(),
            kind: ContextItemKind::Artifact,
            title: id.to_owned(),
            path: Some(format!("{id}.md")),
            language: None,
            snippet: None,
            citation: GraphCitation {
                artifact_path: format!("{id}.md"),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
                content_hash: format!("sha256:{}", "0".repeat(64)),
                artifact_snapshot_id: format!("snp_{}", "0".repeat(64)),
            },
            score: 1,
            provenance: FactProvenance::Deterministic,
            confidence: 1.0,
            trust_state: "direct-evidence",
            source_boundary: "untrusted-project-evidence",
        }
    }

    #[test]
    fn status_is_network_free_and_reports_missing_or_corrupt_files() {
        let directory = tempfile::tempdir().unwrap();
        let absent = directory.path().join("absent");
        assert!(matches!(
            semantic_model_status_at(&absent),
            SemanticModelStatus::Uninstalled { .. }
        ));

        let corrupt = directory.path().join("corrupt");
        fs::create_dir(&corrupt).unwrap();
        set_private_directory_permissions(&corrupt).unwrap();
        fs::write(corrupt.join("config.json"), b"wrong").unwrap();
        set_private_file_permissions(&corrupt.join("config.json")).unwrap();
        assert!(matches!(
            semantic_model_status_at(&corrupt),
            SemanticModelStatus::Corrupt { .. }
        ));
    }

    #[test]
    fn reciprocal_rank_fusion_is_deterministic_and_uses_stable_ids_for_ties() {
        let first = reciprocal_rank_fusion(
            vec![item("lexical"), item("shared")],
            vec![item("semantic"), item("shared")],
        );
        let second = reciprocal_rank_fusion(
            vec![item("lexical"), item("shared")],
            vec![item("semantic"), item("shared")],
        );
        assert_eq!(first, second);
        assert_eq!(first[0].id, "shared");
        assert_eq!(first[1].id, "lexical");
        assert_eq!(first[2].id, "semantic");
    }

    #[test]
    fn semantic_index_validation_rejects_binding_and_embedding_mismatches() {
        let binding = SemanticIndexBinding {
            schema_version: SEMANTIC_INDEX_SCHEMA_VERSION,
            project_id: "prj_00000000000000000000000000000000".to_owned(),
            artifact_snapshot_id: format!("snp_{}", "0".repeat(64)),
            graph_snapshot_id: format!("grf_{}", "1".repeat(64)),
            model_id: SEMANTIC_MODEL_ID.to_owned(),
            model_revision: SEMANTIC_MODEL_REVISION.to_owned(),
            embedding_dimension: SEMANTIC_MODEL_DIMENSION,
        };
        let index = SemanticIndexManifest {
            binding: binding.clone(),
            entries: vec![SemanticIndexEntry {
                id: "artifact:a.md".to_owned(),
                kind: ContextItemKind::Artifact,
                title: "a.md".to_owned(),
                path: Some("a.md".to_owned()),
                language: None,
                citation: GraphCitation {
                    artifact_path: "a.md".to_owned(),
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 1,
                    content_hash: format!("sha256:{}", "0".repeat(64)),
                    artifact_snapshot_id: binding.artifact_snapshot_id.clone(),
                },
                provenance: FactProvenance::Deterministic,
                confidence: 1.0,
                embedding: vec![0.0; SEMANTIC_MODEL_DIMENSION],
            }],
        };
        assert!(validate_semantic_index(&index, &binding).is_ok());
        let mut mismatched = binding.clone();
        mismatched.graph_snapshot_id = format!("grf_{}", "2".repeat(64));
        assert!(validate_semantic_index(&index, &mismatched).is_err());
        let mut invalid = index;
        invalid.entries[0].embedding.pop();
        assert!(validate_semantic_index(&invalid, &binding).is_err());
    }
}
