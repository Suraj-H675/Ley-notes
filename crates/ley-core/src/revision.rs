use crate::graph::capture_git_state;
use crate::{diagnose_project, GitState, LeyCoreError, SessionProjectRevision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RevisionCompatibility {
    CurrentLineage,
    Ancestor,
    Merged,
    Divergent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionApplicability {
    pub compatibility: RevisionCompatibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRevisionFreshness {
    pub live_git_checked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracked_worktree_changes: Option<usize>,
    pub capture_compatibility: RevisionCompatibility,
    pub captured_head_matches_current: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_branch_matches_current: Option<bool>,
}

pub(crate) fn estimate_revision_freshness_tokens(freshness: &ProjectRevisionFreshness) -> usize {
    estimate_serialized_revision_tokens(freshness)
}

pub(crate) fn estimate_revision_applicability_tokens(
    applicability: &RevisionApplicability,
) -> usize {
    estimate_serialized_revision_tokens(applicability)
}

fn estimate_serialized_revision_tokens(value: &impl Serialize) -> usize {
    // Context budgeting elsewhere in Ley uses a conservative four-characters-per-token
    // approximation. Count the actual bounded JSON fields here so a long but valid branch name
    // cannot make revision metadata escape the requested budget.
    let serialized = serde_json::to_vec(value).expect("revision metadata is serializable");
    8usize.saturating_add(serialized.len().div_ceil(4))
}

pub(crate) struct RevisionResolver {
    project_root: PathBuf,
    current: Option<GitState>,
    shallow_repository: bool,
    cache: BTreeMap<(String, Option<String>), RevisionCompatibility>,
    freshness: ProjectRevisionFreshness,
}

impl RevisionResolver {
    pub(crate) fn new(
        project_start: impl AsRef<Path>,
        captured: Option<&GitState>,
    ) -> Result<Self, LeyCoreError> {
        let diagnostic = diagnose_project(project_start)?;
        // Live Git is an optional freshness beacon, not retrieval authority. Any Git-specific
        // failure (missing binary, unsupported repository state, oversized/non-UTF-8 status)
        // degrades to unknown instead of making already-captured memory unavailable.
        let current = capture_git_state(&diagnostic.root).ok().flatten();
        let shallow_repository = current
            .as_ref()
            .is_some_and(|_| git_is_shallow_repository(&diagnostic.root));
        let captured_head = captured.and_then(|git| git.head.clone());
        let captured_branch = captured.and_then(|git| git.branch.clone());
        let current_head = current.as_ref().and_then(|git| git.head.clone());
        let current_branch = current.as_ref().and_then(|git| git.branch.clone());
        let capture_compatibility = classify_revision(
            &diagnostic.root,
            captured_head.as_deref(),
            captured_branch.as_deref(),
            current_head.as_deref(),
            current_branch.as_deref(),
            shallow_repository,
        );
        let mut cache = BTreeMap::new();
        if let Some(captured_head) = captured_head.clone() {
            cache.insert(
                (captured_head, captured_branch.clone()),
                capture_compatibility,
            );
        }
        let captured_head_matches_current =
            captured_head.is_some() && captured_head == current_head;
        let captured_branch_matches_current = match (&captured_branch, &current_branch) {
            (Some(captured), Some(current)) => Some(captured == current),
            _ => None,
        };
        let freshness = ProjectRevisionFreshness {
            live_git_checked: current.is_some(),
            captured_head,
            captured_branch,
            current_head,
            current_branch,
            tracked_worktree_changes: current.as_ref().map(|git| git.changes.len()),
            capture_compatibility,
            captured_head_matches_current,
            captured_branch_matches_current,
        };
        Ok(Self {
            project_root: diagnostic.root,
            current,
            shallow_repository,
            cache,
            freshness,
        })
    }

    pub(crate) fn freshness(&self) -> &ProjectRevisionFreshness {
        &self.freshness
    }

    pub(crate) fn capture_applicability(&self) -> Option<RevisionApplicability> {
        (self.freshness.captured_head.is_some() || self.freshness.captured_branch.is_some()).then(
            || RevisionApplicability {
                compatibility: self.freshness.capture_compatibility,
                captured_head: self.freshness.captured_head.clone(),
                captured_branch: self.freshness.captured_branch.clone(),
            },
        )
    }

    pub(crate) fn applicability(
        &mut self,
        revision: &SessionProjectRevision,
    ) -> RevisionApplicability {
        let compatibility = match (&revision.head, &self.current) {
            (Some(captured_head), Some(current)) => {
                let key = (captured_head.clone(), revision.branch.clone());
                if let Some(compatibility) = self.cache.get(&key) {
                    *compatibility
                } else {
                    let compatibility = classify_revision(
                        &self.project_root,
                        Some(captured_head),
                        revision.branch.as_deref(),
                        current.head.as_deref(),
                        current.branch.as_deref(),
                        self.shallow_repository,
                    );
                    self.cache.insert(key, compatibility);
                    compatibility
                }
            }
            _ => RevisionCompatibility::Unknown,
        };
        RevisionApplicability {
            compatibility,
            captured_head: revision.head.clone(),
            captured_branch: revision.branch.clone(),
        }
    }
}

fn classify_revision(
    project_root: &Path,
    captured_head: Option<&str>,
    captured_branch: Option<&str>,
    current_head: Option<&str>,
    current_branch: Option<&str>,
    shallow_repository: bool,
) -> RevisionCompatibility {
    let (Some(captured_head), Some(current_head)) = (captured_head, current_head) else {
        return RevisionCompatibility::Unknown;
    };
    if !valid_git_object_id(captured_head) || !valid_git_object_id(current_head) {
        return RevisionCompatibility::Unknown;
    }
    if captured_head == current_head {
        return RevisionCompatibility::CurrentLineage;
    }
    match git_is_ancestor(project_root, captured_head, current_head) {
        Some(true) => {
            if matches!((captured_branch, current_branch), (Some(left), Some(right)) if left != right)
            {
                RevisionCompatibility::Merged
            } else {
                RevisionCompatibility::Ancestor
            }
        }
        Some(false) if !shallow_repository => RevisionCompatibility::Divergent,
        Some(false) | None => RevisionCompatibility::Unknown,
    }
}

fn git_is_ancestor(project_root: &Path, ancestor: &str, descendant: &str) -> Option<bool> {
    let status = sanitized_git_command(project_root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    match status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

fn git_is_shallow_repository(project_root: &Path) -> bool {
    let Ok(output) = sanitized_git_command(project_root)
        .args(["rev-parse", "--is-shallow-repository"])
        .stderr(Stdio::null())
        .output()
    else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    output.stdout == b"true\n"
}

fn valid_git_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sanitized_git_command(project_root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("core.fsmonitor=false")
        .arg("-c")
        .arg("core.untrackedCache=false")
        .arg("-C")
        .arg(project_root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("LC_ALL", "C")
        .stdin(Stdio::null());
    for variable in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CEILING_DIRECTORIES",
        "GIT_DISCOVERY_ACROSS_FILESYSTEM",
        "GIT_TRACE",
        "GIT_TRACE2",
        "GIT_TRACE2_EVENT",
        "GIT_TRACE2_PERF",
        "GIT_TRACE_PERFORMANCE",
        "GIT_TRACE_SETUP",
        "GIT_TRACE_PACKET",
        "GIT_TRACE_CURL",
        "GIT_REDIRECT_STDERR",
    ] {
        command.env_remove(variable);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn git(project: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(project)
            .args(args)
            .env("LC_ALL", "C")
            .output()
            .unwrap();
        assert!(output.status.success(), "git {:?} failed", args);
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn commit(project: &Path, file: &str, body: &str, message: &str) -> String {
        fs::write(project.join(file), body).unwrap();
        git(project, &["add", file]);
        git(
            project,
            &[
                "-c",
                "user.name=Ley Test",
                "-c",
                "user.email=ley@example.invalid",
                "commit",
                "-m",
                message,
            ],
        );
        git(project, &["rev-parse", "HEAD"])
    }

    #[test]
    fn revision_compatibility_uses_git_ancestry_without_recency_guessing() {
        let root = tempdir().unwrap();
        let project = root.path();
        git(project, &["init", "-b", "main"]);
        let base = commit(project, "state.txt", "base\n", "base");
        let same = classify_revision(
            project,
            Some(&base),
            Some("main"),
            Some(&base),
            Some("main"),
            false,
        );
        assert_eq!(same, RevisionCompatibility::CurrentLineage);

        let current = commit(project, "state.txt", "current\n", "current");
        assert_eq!(
            classify_revision(
                project,
                Some(&base),
                Some("main"),
                Some(&current),
                Some("main"),
                false,
            ),
            RevisionCompatibility::Ancestor
        );
        assert_eq!(
            classify_revision(
                project,
                Some(&base),
                Some("feature"),
                Some(&current),
                Some("main"),
                false,
            ),
            RevisionCompatibility::Merged
        );
    }

    #[test]
    fn divergent_and_unknown_revision_states_fail_closed() {
        let root = tempdir().unwrap();
        let project = root.path();
        git(project, &["init", "-b", "main"]);
        let base = commit(project, "state.txt", "base\n", "base");
        git(project, &["checkout", "-b", "experiment"]);
        let experimental = commit(project, "experiment.txt", "experiment\n", "experiment");
        git(project, &["checkout", "main"]);
        let current = commit(project, "main.txt", "main\n", "main");
        assert_ne!(experimental, current);
        assert_eq!(
            classify_revision(
                project,
                Some(&experimental),
                Some("experiment"),
                Some(&current),
                Some("main"),
                false,
            ),
            RevisionCompatibility::Divergent
        );
        assert_eq!(
            classify_revision(
                project,
                Some(&experimental),
                Some("experiment"),
                Some(&current),
                Some("main"),
                true,
            ),
            RevisionCompatibility::Unknown
        );
        assert_eq!(
            classify_revision(project, None, None, Some(&base), Some("main"), false),
            RevisionCompatibility::Unknown
        );
    }

    #[test]
    fn revision_token_estimates_scale_with_long_valid_branch_metadata() {
        let branch = "b".repeat(1_024);
        let applicability = RevisionApplicability {
            compatibility: RevisionCompatibility::Ancestor,
            captured_head: Some("a".repeat(40)),
            captured_branch: Some(branch.clone()),
        };
        let applicability_bytes = serde_json::to_vec(&applicability).unwrap().len();
        assert_eq!(
            estimate_revision_applicability_tokens(&applicability),
            8 + applicability_bytes.div_ceil(4)
        );
        assert!(estimate_revision_applicability_tokens(&applicability) > 250);

        let freshness = ProjectRevisionFreshness {
            live_git_checked: true,
            captured_head: Some("a".repeat(40)),
            captured_branch: Some(branch.clone()),
            current_head: Some("b".repeat(40)),
            current_branch: Some(branch),
            tracked_worktree_changes: Some(0),
            capture_compatibility: RevisionCompatibility::Ancestor,
            captured_head_matches_current: false,
            captured_branch_matches_current: Some(true),
        };
        let freshness_bytes = serde_json::to_vec(&freshness).unwrap().len();
        assert_eq!(
            estimate_revision_freshness_tokens(&freshness),
            8 + freshness_bytes.div_ceil(4)
        );
        assert!(estimate_revision_freshness_tokens(&freshness) > 500);
    }
}
