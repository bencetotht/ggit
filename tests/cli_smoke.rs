use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn git(path: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    git(path, &["init"]);
    git(path, &["config", "user.email", "test@example.com"]);
    git(path, &["config", "user.name", "Test User"]);
    fs::write(path.join("README.md"), "test\n").unwrap();
    git(path, &["add", "."]);
    git(path, &["commit", "-m", "initial"]);
}

fn ggit(db_path: &Path) -> Command {
    let mut cmd = Command::cargo_bin("ggit").unwrap();
    cmd.env("GGIT_DB_PATH", db_path);
    cmd
}

#[test]
fn scan_list_status_pull_dry_run_and_clear() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("ggit.db");
    let repo = tmp.path().join("example");
    init_repo(&repo);

    ggit(&db_path)
        .args(["scan", tmp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("example"));

    ggit(&db_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("example"));

    ggit(&db_path)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("clean"));

    ggit(&db_path)
        .args(["pull", "--filter", "example", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would pull 1 repositories"));

    ggit(&db_path).args(["clear", "--yes"]).assert().success();
    ggit(&db_path)
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No repositories are tracked yet"));
}
