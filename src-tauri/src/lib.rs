use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultFile {
    path: String,
    content: String,
    created_at: u64,
    updated_at: u64,
}

fn unix_millis(value: Result<SystemTime, std::io::Error>) -> u64 {
    value
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn canonical_vault(vault_path: &str) -> Result<PathBuf, String> {
    let root =
        fs::canonicalize(vault_path).map_err(|error| format!("Cannot open vault: {error}"))?;
    if !root.is_dir() {
        return Err("The selected vault is not a folder".into());
    }
    Ok(root)
}

fn safe_relative(relative_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative_path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err("Vault paths must be relative".into());
    }
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Vault path contains an unsafe segment".into());
    }
    Ok(path.to_path_buf())
}

fn markdown_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = safe_relative(relative_path)?;
    if relative.extension().and_then(|part| part.to_str()) != Some("md") {
        return Err("Ley can only mutate Markdown notes".into());
    }
    Ok(root.join(relative))
}

fn visible_entry(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if entry.depth() == 0 {
        return true;
    }
    !name.starts_with('.') && name != "node_modules"
}

#[tauri::command]
fn scan_vault(vault_path: String) -> Result<Vec<VaultFile>, String> {
    let root = canonical_vault(&vault_path)?;
    let mut files = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(visible_entry)
    {
        let entry = entry.map_err(|error| format!("Failed to scan vault: {error}"))?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|part| part.to_str()) != Some("md")
        {
            continue;
        }
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| "A scanned file escaped the vault root")?
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Cannot inspect {relative}: {error}"))?;
        let content =
            fs::read_to_string(path).map_err(|error| format!("Cannot read {relative}: {error}"))?;
        files.push(VaultFile {
            path: relative,
            content,
            created_at: unix_millis(metadata.created()),
            updated_at: unix_millis(metadata.modified()),
        });
    }

    files.sort_by(|left, right| left.path.to_lowercase().cmp(&right.path.to_lowercase()));
    Ok(files)
}

#[tauri::command]
fn write_vault_file(
    vault_path: String,
    relative_path: String,
    content: String,
) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let target = markdown_path(&root, &relative_path)?;
    let parent = target.parent().ok_or("The note has no parent folder")?;
    fs::create_dir_all(parent).map_err(|error| format!("Cannot create note folder: {error}"))?;

    let temp_name = format!(
        ".{}.ley-write",
        target.file_name().unwrap_or_default().to_string_lossy()
    );
    let temp = parent.join(temp_name);
    let mut file =
        fs::File::create(&temp).map_err(|error| format!("Cannot stage note: {error}"))?;
    file.write_all(content.as_bytes())
        .map_err(|error| format!("Cannot write note: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Cannot flush note: {error}"))?;
    fs::rename(&temp, &target).map_err(|error| format!("Cannot replace note: {error}"))?;
    Ok(())
}

#[tauri::command]
fn rename_vault_file(vault_path: String, from: String, to: String) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let source = markdown_path(&root, &from)?;
    let target = markdown_path(&root, &to)?;
    if target.exists() {
        return Err(format!("A note already exists at {to}"));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Cannot create destination folder: {error}"))?;
    }
    fs::rename(source, target).map_err(|error| format!("Cannot rename note: {error}"))
}

#[tauri::command]
fn trash_vault_file(vault_path: String, relative_path: String) -> Result<String, String> {
    let root = canonical_vault(&vault_path)?;
    let source = markdown_path(&root, &relative_path)?;
    if !source.exists() {
        return Err("The note no longer exists".into());
    }
    let trash = root.join(".trash");
    fs::create_dir_all(&trash).map_err(|error| format!("Cannot create .trash: {error}"))?;
    let original = source
        .file_name()
        .ok_or("The note has no filename")?
        .to_string_lossy();
    let mut candidate = trash.join(original.as_ref());
    let mut suffix = 2;
    while candidate.exists() {
        let stem = Path::new(original.as_ref())
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        candidate = trash.join(format!("{stem} {suffix}.md"));
        suffix += 1;
    }
    fs::rename(source, &candidate)
        .map_err(|error| format!("Cannot move note to .trash: {error}"))?;
    Ok(candidate
        .strip_prefix(&root)
        .unwrap_or(&candidate)
        .to_string_lossy()
        .replace('\\', "/"))
}

#[tauri::command]
fn create_vault_folder(vault_path: String, relative_path: String) -> Result<(), String> {
    let root = canonical_vault(&vault_path)?;
    let relative = safe_relative(&relative_path)?;
    fs::create_dir_all(root.join(relative))
        .map_err(|error| format!("Cannot create folder: {error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_vault,
            write_vault_file,
            rename_vault_file,
            trash_vault_file,
            create_vault_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ley");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_vault_lifecycle_is_real_and_confined() {
        let root = std::env::temp_dir().join(format!("ley-native-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let vault = root.to_string_lossy().to_string();

        write_vault_file(
            vault.clone(),
            "projects/First note.md".into(),
            "---\ntags: [test]\n---\n# First\n\nLinked to [[Second]].".into(),
        )
        .unwrap();
        assert!(root.join("projects/First note.md").is_file());

        let scanned = scan_vault(vault.clone()).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].path, "projects/First note.md");
        assert!(scanned[0].content.contains("[[Second]]"));

        rename_vault_file(
            vault.clone(),
            "projects/First note.md".into(),
            "projects/Renamed.md".into(),
        )
        .unwrap();
        assert!(!root.join("projects/First note.md").exists());
        assert!(root.join("projects/Renamed.md").is_file());

        let trashed = trash_vault_file(vault.clone(), "projects/Renamed.md".into()).unwrap();
        assert_eq!(trashed, ".trash/Renamed.md");
        assert!(root.join(".trash/Renamed.md").is_file());
        assert!(scan_vault(vault.clone()).unwrap().is_empty());

        assert!(write_vault_file(vault, "../escape.md".into(), "nope".into()).is_err());
        assert!(!root.parent().unwrap().join("escape.md").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
