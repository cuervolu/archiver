use archiver_core::Archiver;

mod helpers;
use helpers::{setup_test_env, setup_tracing};

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

    let empty_project_path = settings.projects_dir.join("empty_project");
    assert!(
        empty_project_path.exists(),
        "Empty project should be ignored and remain"
    );

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

#[test]
fn it_ignores_excluded_projects() {
    setup_tracing();
    let (_temp_dir, mut settings) = setup_test_env();

    let excluded_project_name = "old_project";
    settings.exclude.push(excluded_project_name.to_string());

    let archiver = Archiver::new(settings.clone());

    archiver.run_archive_process(false).unwrap();

    let excluded_project_path = settings.projects_dir.join(excluded_project_name);
    assert!(
        excluded_project_path.exists(),
        "Excluded project should not have been moved."
    );
    assert!(!settings.archive_dir.join(excluded_project_name).exists());

    let log_content = archiver.get_archive_records().unwrap();
    assert!(
        log_content.is_empty(),
        "The archive log should be empty when the only inactive project is excluded"
    );
}
