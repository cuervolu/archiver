use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_paths_command_runs_successfully() {
    let mut cmd = Command::cargo_bin("archiver").unwrap();
    
    cmd.arg("paths");
    
    cmd.assert()
        .success() 
        .stdout(predicate::str::contains("Configuration paths:"))
        .stdout(predicate::str::contains("Projects directory:"))
        .stdout(predicate::str::contains("Archive directory:"));
}
