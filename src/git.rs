use crate::error::GgitError;
use crate::status::RepoStatus;
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug)]
pub struct PullResult {
    pub updated: bool,
    pub output: String,
}

pub fn git_version() -> Result<String> {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .map_err(|_| GgitError::MissingGit)?;
    command_text(output, "git --version", Path::new("."))
}

pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .current_dir(path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn repo_remote_url(path: &Path) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .with_context(|| format!("could not run git remote in {}", path.display()))?;
    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else {
        Ok(None)
    }
}

pub fn current_branch(path: &Path) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(path)
        .output()
        .with_context(|| format!("could not run git branch in {}", path.display()))?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!branch.is_empty()).then_some(branch))
    } else {
        Ok(None)
    }
}

pub fn status_snapshot(path: &Path) -> Result<RepoStatus> {
    if !path.exists() {
        return Ok(RepoStatus::missing());
    }
    if !is_git_repo(path) {
        return Ok(RepoStatus::not_git());
    }
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "--branch"])
        .current_dir(path)
        .output()
        .with_context(|| format!("could not run git status in {}", path.display()))?;
    let text = command_text(output, "git status --porcelain=v1 --branch", path)?;
    Ok(RepoStatus::parse(&text))
}

pub fn pull_ff_only(path: &Path) -> Result<PullResult> {
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(path)
        .output()
        .with_context(|| format!("could not run git pull in {}", path.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    if !output.status.success() {
        return Err(anyhow!(
            "git pull --ff-only failed in {}: {}",
            path.display(),
            combined.trim()
        ));
    }

    let lower = combined.to_lowercase();
    let up_to_date = lower.contains("already up to date") || lower.contains("already up-to-date");
    Ok(PullResult {
        updated: !up_to_date,
        output: combined,
    })
}

pub fn abort_merge_or_rebase_if_needed(path: &Path) -> Result<()> {
    let git_dir = git_dir(path)?;
    if git_dir.join("MERGE_HEAD").exists() {
        let _ = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(path)
            .output();
    }
    if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
        let _ = Command::new("git")
            .args(["rebase", "--abort"])
            .current_dir(path)
            .output();
    }
    Ok(())
}

fn git_dir(path: &Path) -> Result<std::path::PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(path)
        .output()
        .with_context(|| format!("could not inspect git dir in {}", path.display()))?;
    let text = command_text(output, "git rev-parse --git-dir", path)?;
    let git_dir = std::path::PathBuf::from(text.trim());
    if git_dir.is_absolute() {
        Ok(git_dir)
    } else {
        Ok(path.join(git_dir))
    }
}

fn command_text(output: Output, command: &str, path: &Path) -> Result<String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!(
            "{command} failed in {}: {}",
            path.display(),
            stderr.trim()
        ))
    }
}
