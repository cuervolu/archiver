use archiver_core::{config::Settings, Archiver};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

/// Helper function to initialize the tracing subscriber for tests.
fn setup_tracing() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
}

/// Helper function to set up a test environment with temporary directories
/// and fake git repositories.
fn setup_test_env() -> (tempfile::TempDir, Settings) {
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

#[test]
fn it_archives_only_inactive_projects_on_real_run() {
    setup_tracing();
    let (_temp_dir, settings) = setup_test_env();
    let archiver = Archiver::new(settings.clone());

    let result = archiver.run_archive_process(false);
    assert!(result.is_ok());

    let old_project_original_path = settings.projects_dir.join("old_project");
    let old_project_archived_path = settings.archive_dir.join("old_project");
    assert!(!old_project_original_path.exists());
    assert!(old_project_archived_path.exists());

    let new_project_path = settings.projects_dir.join("new_project");
    assert!(new_project_path.exists());

    let log_content = archiver.get_archive_records().unwrap();
    assert_eq!(log_content.len(), 1);
    assert!(log_content.iter().any(|r| r.name == "old_project"));
}

#[test]
fn it_ignores_empty_repositories_without_commits() {
    setup_tracing();
    let (_temp_dir, settings) = setup_test_env();
    let archiver = Archiver::new(settings.clone());

    let result = archiver.run_archive_process(false);
    assert!(result.is_ok()); // Should not error out

    // Check that the empty project was not moved and is still there
    let empty_project_path = settings.projects_dir.join("empty_project");
    assert!(
        empty_project_path.exists(),
        "Empty project should be ignored and remain"
    );

    // Check that the empty project was not added to the log
    let log_content = archiver.get_archive_records().unwrap();
    assert!(!log_content.iter().any(|r| r.name == "empty_project"));
}

#[test]
fn it_restores_an_archived_project() {
    setup_tracing();
    let (_temp_dir, settings) = setup_test_env();
    let archiver = Archiver::new(settings.clone());
    archiver.run_archive_process(false).unwrap();

    let old_project_original_path = settings.projects_dir.join("old_project");
    let old_project_archived_path = settings.archive_dir.join("old_project");
    assert!(!old_project_original_path.exists());
    assert!(old_project_archived_path.exists());

    let result = archiver.restore_project("old_project");
    assert!(result.is_ok());

    assert!(old_project_original_path.exists());
    assert!(!old_project_archived_path.exists());

    let log_content = archiver.get_archive_records().unwrap();
    assert!(log_content.is_empty());
}
