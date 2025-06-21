use std::fs;
use std::process::Command;
use tempfile::tempdir;
use archiver_core::Settings;

/// Helper function to set up a test environment with temporary directories
/// and fake git repositories.
pub fn setup_test_env() -> (tempfile::TempDir, Settings) {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let projects_dir = temp_dir.path().join("projects");
    let archive_dir = temp_dir.path().join("archive");
    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&archive_dir).unwrap();

    // --- Create a project that SHOULD be archived ---
    let old_project_path = projects_dir.join("old_project");
    fs::create_dir(&old_project_path).unwrap();
    init_git_repo_with_date(&old_project_path, "old commit", "2023-01-01T12:00:00Z");

    // --- Create a project that SHOULD NOT be archived ---
    let new_project_path = projects_dir.join("new_project");
    fs::create_dir(&new_project_path).unwrap();
    let now_iso = chrono::Utc::now().to_rfc3339();
    init_git_repo_with_date(&new_project_path, "new commit", &now_iso);

    // --- Create an empty project that should be ignored ---
    let empty_project_path = projects_dir.join("empty_project");
    fs::create_dir(&empty_project_path).unwrap();
    Command::new("git")
        .arg("init")
        .current_dir(&empty_project_path)
        .output()
        .unwrap();

    let settings = Settings {
        projects_dir,
        archive_dir,
        inactivity_days: 30,
        ..Default::default()
    };

    (temp_dir, settings)
}

/// Helper function to initialize the tracing subscriber for tests.
pub fn setup_tracing() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
}

/// Helper to initialize a git repo and create a commit with a specific date.
fn init_git_repo_with_date(path: &std::path::Path, msg: &str, date: &str) {
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .unwrap();
    fs::write(path.join("file.txt"), msg).unwrap();
    Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(msg)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .current_dir(path)
        .output()
        .unwrap();
}