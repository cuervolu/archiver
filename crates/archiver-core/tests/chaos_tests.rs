use archiver_core::{config::Settings, Archiver, Error};
use std::fs;
use tempfile::tempdir;

mod helpers;
use helpers::setup_test_env;

#[test]
fn core_c_02_it_fails_gracefully_with_corrupt_config() {
    // Create a temporary settings.toml with invalid TOML syntax.
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("settings.toml");
    fs::write(&config_path, "inactivity_days = 'not a number'").unwrap();

    // Manually build a config object pointing to our corrupt file.
    // This avoids using Settings::new() which depends on global state (like HOME).
    let result = config::Config::builder()
        .add_source(config::File::from(config_path))
        .build()
        .and_then(|c| c.try_deserialize::<Settings>());
    
    assert!(result.is_err(), "Expected deserialization to fail");
    assert!(
        matches!(result.err().unwrap(), config::ConfigError::Type { .. }),
        "Expected a Type error from config-rs"
    );
}

#[test]
fn core_fs_02_it_fails_gracefully_with_read_only_archive_dir() {
    // Setup a normal environment, but then make the archive dir read-only.
    let (_temp_dir, settings) = setup_test_env();
    let archiver = Archiver::new(settings.clone());

    // Save original permissions to restore them later, avoiding clippy lints.
    let original_perms = fs::metadata(&settings.archive_dir).unwrap().permissions();
    let mut readonly_perms = original_perms.clone();
    readonly_perms.set_readonly(true);
    fs::set_permissions(&settings.archive_dir, readonly_perms).unwrap();
    
    let result = archiver.run_archive_process(false);
    
    assert!(result.is_err(), "Expected archiving to fail due to permissions");
    let error = result.err().unwrap();
    assert!(matches!(error, Error::Io(_)), "Expected an I/O error");

    // Cleanup: Restore original permissions to ensure the temp dir can be deleted.
    fs::set_permissions(&settings.archive_dir, original_perms).unwrap();
}

#[test]
fn core_l_01_it_fails_gracefully_with_corrupt_log_file() {
    // Setup a normal env, then write garbage to archive.json
    let (_temp_dir, settings) = setup_test_env();
    fs::write(
        settings.archive_dir.join("archive.json"),
        "{not_valid_json: true}",
    )
        .unwrap();

    let archiver = Archiver::new(settings);
    
    // This action will try to read the corrupt log before appending to it.
    let result = archiver.run_archive_process(false);
    
    assert!(result.is_err(), "Expected archiving to fail due to corrupt log");
    let error = result.err().unwrap();
    assert!(matches!(error, Error::Json(_)), "Expected a JSON deserialization error");
}
