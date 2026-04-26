use crate::db::Repository;
use crate::git;
use anyhow::Result;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_depth: Option<usize>,
    pub include_hidden: bool,
}

#[derive(Debug)]
pub struct ScanResult {
    pub repositories: Vec<Repository>,
    pub pruned_directories: usize,
}

const PRUNED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "env",
    ".cache",
    ".cache-loader",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".parcel-cache",
    "coverage",
    "vendor",
    "Pods",
    "DerivedData",
    ".gradle",
    ".idea",
    ".vscode",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".tox",
];

pub fn scan_repositories(root: &Path, options: &ScanOptions) -> Result<ScanResult> {
    let mut repositories = Vec::new();
    let mut pruned_directories = 0usize;
    let mut walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(options.max_depth.unwrap_or(usize::MAX))
        .into_iter();

    while let Some(entry) = walker.next() {
        let entry = entry?;
        if entry.depth() > 0 && entry.file_type().is_dir() && should_prune(&entry, options) {
            pruned_directories += 1;
            walker.skip_current_dir();
            continue;
        }

        if entry.file_type().is_dir() && is_repo_dir(entry.path()) {
            repositories.push(repository_from_path(entry.path())?);
            walker.skip_current_dir();
        }
    }

    repositories.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ScanResult {
        repositories,
        pruned_directories,
    })
}

fn is_repo_dir(path: &Path) -> bool {
    path.join(".git").exists()
}

fn repository_from_path(path: &Path) -> Result<Repository> {
    let canonical = path.canonicalize()?;
    let name = canonical
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("repository")
        .to_string();
    Ok(Repository {
        id: None,
        name,
        remote_url: git::repo_remote_url(&canonical)?,
        current_branch: git::current_branch(&canonical)?,
        path: canonical,
    })
}

fn should_prune(entry: &DirEntry, options: &ScanOptions) -> bool {
    let name = entry.file_name().to_string_lossy();
    if PRUNED_DIRS.contains(&name.as_ref()) {
        return true;
    }
    !options.include_hidden && name.starts_with('.')
}

#[allow(dead_code)]
pub fn pruned_dirs() -> &'static [&'static str] {
    PRUNED_DIRS
}

#[allow(dead_code)]
fn normalize_path(path: PathBuf) -> PathBuf {
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_repo(path: &Path) {
        fs::create_dir_all(path.join(".git")).unwrap();
    }

    #[test]
    fn finds_repos_and_skips_children() {
        let dir = tempdir().unwrap();
        let outer = dir.path().join("outer");
        let inner = outer.join("inner");
        make_repo(&outer);
        make_repo(&inner);

        let result = scan_repositories(
            dir.path(),
            &ScanOptions {
                max_depth: None,
                include_hidden: false,
            },
        )
        .unwrap();

        assert_eq!(result.repositories.len(), 1);
        assert_eq!(result.repositories[0].name, "outer");
    }

    #[test]
    fn respects_max_depth() {
        let dir = tempdir().unwrap();
        make_repo(&dir.path().join("a").join("b").join("repo"));

        let result = scan_repositories(
            dir.path(),
            &ScanOptions {
                max_depth: Some(2),
                include_hidden: false,
            },
        )
        .unwrap();

        assert!(result.repositories.is_empty());
    }

    #[test]
    fn prunes_known_heavy_dirs() {
        let dir = tempdir().unwrap();
        make_repo(&dir.path().join("node_modules").join("dep"));
        make_repo(&dir.path().join(".cache").join("dep"));

        let result = scan_repositories(
            dir.path(),
            &ScanOptions {
                max_depth: None,
                include_hidden: false,
            },
        )
        .unwrap();

        assert!(result.repositories.is_empty());
        assert_eq!(result.pruned_directories, 2);
    }

    #[test]
    fn finds_git_file_repositories() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("worktree-style");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join(".git"), "gitdir: ../real-git-dir\n").unwrap();

        let result = scan_repositories(
            dir.path(),
            &ScanOptions {
                max_depth: None,
                include_hidden: false,
            },
        )
        .unwrap();

        assert_eq!(result.repositories.len(), 1);
        assert_eq!(result.repositories[0].name, "worktree-style");
    }
}
