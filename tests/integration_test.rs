//! Integration tests for jflow
//!
//! These tests create real jj repositories and test the CLI end-to-end.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

/// Helper to create a jj git repo in a temp directory
fn create_jj_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    // Initialize jj git repo
    std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to init jj repo");

    // Set up user for commits
    std::process::Command::new("jj")
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to set user.name");

    std::process::Command::new("jj")
        .args(["config", "set", "--repo", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to set user.email");

    dir
}

/// Helper to create a jj repo with a local bare git remote as "origin"
fn create_jj_repo_with_remote() -> (tempfile::TempDir, tempfile::TempDir) {
    // Create a bare git repo to act as "origin"
    let remote_dir = tempdir().unwrap();
    std::process::Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote_dir.path())
        .output()
        .expect("Failed to init bare git repo");

    // Create the jj repo
    let repo_dir = tempdir().unwrap();
    std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to init jj repo");

    // Set up user for commits
    std::process::Command::new("jj")
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to set user.name");

    std::process::Command::new("jj")
        .args(["config", "set", "--repo", "user.email", "test@test.com"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to set user.email");

    // Add the bare repo as origin remote
    let remote_path = remote_dir.path().to_str().unwrap();
    std::process::Command::new("jj")
        .args(["git", "remote", "add", "origin", remote_path])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to add remote");

    // Create an initial commit with a description and push as main
    std::process::Command::new("jj")
        .args(["describe", "-m", "Initial commit"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to describe initial commit");

    // Create main bookmark at the initial commit
    std::process::Command::new("jj")
        .args(["bookmark", "create", "main", "-r", "@"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to create main bookmark");

    // Push main to origin
    std::process::Command::new("jj")
        .args(["git", "push", "--bookmark", "main"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to push main");

    // Create a new empty commit for working
    std::process::Command::new("jj")
        .args(["new"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to create new commit");

    (repo_dir, remote_dir)
}

/// Helper to create .jflow.toml config
fn create_jflow_config(dir: &std::path::Path) {
    let config = r#"
[remote]
name = "origin"
primary = "main"

[github]
push_style = "squash"
"#;
    fs::write(dir.join(".jflow.toml"), config).unwrap();
}

#[test]
fn test_jf_version() {
    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("jf"));
}

#[test]
fn test_jf_help() {
    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Beautiful workflow tool"));
}

#[test]
fn test_jf_init_creates_config() {
    let dir = create_jj_repo();

    let mut cmd = Command::cargo_bin("jf").unwrap();
    // Use --local to force creating local config even if global exists
    cmd.args(["init", "--defaults", "--local"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .jflow.toml"));

    // Verify config was created
    assert!(dir.path().join(".jflow.toml").exists());

    // Verify config content
    let content = fs::read_to_string(dir.path().join(".jflow.toml")).unwrap();
    assert!(content.contains("[remote]"));
    assert!(content.contains("primary"));
}

#[test]
fn test_jf_init_fails_outside_jj_repo() {
    let dir = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["init", "--defaults"])
        .current_dir(dir.path())
        .assert()
        .success() // Command succeeds but prints error
        .stderr(predicate::str::contains("Not in a jj repository"));
}

#[test]
fn test_jf_init_fails_if_config_exists() {
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["init", "--defaults"])
        .current_dir(dir.path())
        .assert()
        .success() // Command succeeds but prints error
        .stderr(predicate::str::contains(".jflow.toml already exists"));
}

#[test]
fn test_jf_status_shows_stack() {
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["status"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"));
}

#[test]
fn test_jf_status_default_command() {
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    // Running jf with no args should run status
    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"));
}

// Tests that require a remote repository use create_jj_repo_with_remote()

#[test]
fn test_jf_land_dry_run_with_remote() {
    let (repo_dir, _remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["land", "--dry-run"])
        .current_dir(repo_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No merged PRs found"));
}

#[test]
fn test_jf_push_dry_run_with_remote() {
    let (repo_dir, _remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    // Create a change with description to push
    std::process::Command::new("jj")
        .args(["describe", "-m", "Test change"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to describe change");

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["push", "--dry-run"])
        .current_dir(repo_dir.path())
        .assert()
        .success();
}

#[test]
fn test_jf_pull_with_remote() {
    let (repo_dir, _remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["pull"])
        .current_dir(repo_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetching"));
}

#[test]
fn test_jf_status_with_changes() {
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    // Create some changes
    std::process::Command::new("jj")
        .args(["new", "-m", "First change"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to create change");

    std::process::Command::new("jj")
        .args(["new", "-m", "Second change"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to create change");

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"))
        .stdout(predicate::str::contains("commits"));
}

// === Edge Case Integration Tests ===

#[test]
fn test_jf_status_empty_repo() {
    // A completely fresh jj repo with no commits
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"));
}

#[test]
fn test_jf_status_with_unicode_description() {
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    // Create a change with unicode description
    std::process::Command::new("jj")
        .args(["describe", "-m", "添加功能 🎉"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to describe change");

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"));
}

#[test]
fn test_jf_status_with_special_chars_in_description() {
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    // Create a change with special characters
    std::process::Command::new("jj")
        .args(["describe", "-m", "Fix \"bug\" with 'quotes' & <angles>"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to describe change");

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn test_jf_init_with_custom_primary() {
    let dir = create_jj_repo();

    // Create a master branch instead of main
    std::process::Command::new("jj")
        .args(["bookmark", "create", "master", "-r", "@"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to create master bookmark");

    let mut cmd = Command::cargo_bin("jf").unwrap();
    // Use --local to force creating local config even if global exists
    cmd.args(["init", "--defaults", "--local"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Verify config was created
    let content = fs::read_to_string(dir.path().join(".jflow.toml")).unwrap();
    // Should detect master as primary branch
    assert!(content.contains("[remote]"));
}

#[test]
fn test_jf_status_with_many_changes() {
    let dir = create_jj_repo();
    create_jflow_config(dir.path());

    // Create 10 changes
    for i in 1..=10 {
        std::process::Command::new("jj")
            .args(["new", "-m", &format!("Change {}", i)])
            .current_dir(dir.path())
            .output()
            .expect("Failed to create change");
    }

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"));
}

#[test]
fn test_jf_push_no_changes() {
    let (repo_dir, _remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    // Current commit is empty (no description)
    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["push", "--dry-run"])
        .current_dir(repo_dir.path())
        .assert()
        .success();
}

#[test]
fn test_jf_land_empty_stack() {
    let (repo_dir, _remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    // No bookmarks to land
    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["land", "--dry-run"])
        .current_dir(repo_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No merged PRs found"));
}

#[test]
fn test_jf_status_works_without_config() {
    // jf status should work even without .jflow.toml (uses defaults)
    let dir = create_jj_repo();

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"));
}

#[test]
fn test_jf_pull_updates_from_remote() {
    let (repo_dir, remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    // Create a second clone that pushes a change
    let clone_dir = tempdir().unwrap();
    std::process::Command::new("git")
        .args(["clone", remote_dir.path().to_str().unwrap(), "."])
        .current_dir(clone_dir.path())
        .output()
        .expect("Failed to clone");

    // Add a commit in the clone
    fs::write(clone_dir.path().join("newfile.txt"), "content").unwrap();
    std::process::Command::new("git")
        .args(["add", "newfile.txt"])
        .current_dir(clone_dir.path())
        .output()
        .expect("Failed to add file");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add newfile"])
        .current_dir(clone_dir.path())
        .output()
        .expect("Failed to commit");

    std::process::Command::new("git")
        .args(["push"])
        .current_dir(clone_dir.path())
        .output()
        .expect("Failed to push");

    // Now pull in the original jj repo
    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.args(["pull"])
        .current_dir(repo_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetching"));
}

#[test]
fn test_jf_with_long_bookmark_name() {
    let (repo_dir, _remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    // Create a change and a very long bookmark name
    std::process::Command::new("jj")
        .args(["describe", "-m", "Test change"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to describe");

    let long_name = "feature/very-long-bookmark-name-that-might-cause-issues-with-display-or-parsing-in-various-scenarios";
    std::process::Command::new("jj")
        .args(["bookmark", "create", long_name, "-r", "@"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to create bookmark");

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(repo_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Your Stack"));
}

#[test]
fn test_jf_with_bookmark_special_chars() {
    let (repo_dir, _remote_dir) = create_jj_repo_with_remote();
    create_jflow_config(repo_dir.path());

    std::process::Command::new("jj")
        .args(["describe", "-m", "Test change"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to describe");

    // Create bookmark with slashes (common pattern)
    std::process::Command::new("jj")
        .args(["bookmark", "create", "feature/add-login", "-r", "@"])
        .current_dir(repo_dir.path())
        .output()
        .expect("Failed to create bookmark");

    let mut cmd = Command::cargo_bin("jf").unwrap();
    cmd.current_dir(repo_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("feature/add-login"));
}
