use std::fs;
use std::path::Path;

pub fn seed_legacy_project(project: &Path, vault: &Path, name: &str) {
    fs::create_dir_all(project).unwrap();
    fs::create_dir_all(vault).unwrap();
    let readme = project.join("README.md");
    if !readme.exists() {
        fs::write(&readme, format!("# {name}\n")).unwrap();
    }
    ley_core::initialize_project(project, Some(name), ley_core::CaptureMode::Structured).unwrap();
    ley_core::ingest_project(project, vault).unwrap();
}
