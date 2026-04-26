use crate::db::Repository;
use crate::git;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    pub branch: String,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub sync: SyncStatus,
    pub worktree_dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Clean,
    NeedsPush(usize),
    NeedsPull(usize),
    Diverged { ahead: usize, behind: usize },
    NoUpstream,
    MissingPath,
    NotGitRepo,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct StatusRow {
    pub name: String,
    pub path: String,
    pub remote: String,
    pub branch: String,
    pub sync: String,
    pub worktree: String,
}

impl RepoStatus {
    pub fn parse(input: &str) -> Self {
        let mut lines = input.lines();
        let header = lines.next().unwrap_or("");
        let branch = parse_branch(header);
        let ahead = parse_marker(header, "ahead");
        let behind = parse_marker(header, "behind");
        let no_upstream = header.starts_with("## ") && !header.contains("...");
        let worktree_dirty = lines.any(|line| !line.trim().is_empty());
        let sync = match (ahead, behind, no_upstream) {
            (Some(a), Some(b), _) => SyncStatus::Diverged {
                ahead: a,
                behind: b,
            },
            (_, _, true) => SyncStatus::NoUpstream,
            (Some(a), None, _) => SyncStatus::NeedsPush(a),
            (None, Some(b), _) => SyncStatus::NeedsPull(b),
            _ => SyncStatus::Clean,
        };

        Self {
            branch,
            ahead,
            behind,
            sync,
            worktree_dirty,
        }
    }

    pub fn missing() -> Self {
        Self {
            branch: "-".to_string(),
            ahead: None,
            behind: None,
            sync: SyncStatus::MissingPath,
            worktree_dirty: false,
        }
    }

    pub fn not_git() -> Self {
        Self {
            branch: "-".to_string(),
            ahead: None,
            behind: None,
            sync: SyncStatus::NotGitRepo,
            worktree_dirty: false,
        }
    }
}

pub fn status_row(repo: &Repository, short: bool) -> StatusRow {
    let status = git::status_snapshot(&repo.path).unwrap_or_else(|_| RepoStatus {
        branch: repo
            .current_branch
            .clone()
            .unwrap_or_else(|| "-".to_string()),
        ahead: None,
        behind: None,
        sync: SyncStatus::Unknown,
        worktree_dirty: false,
    });

    StatusRow {
        name: repo.name.clone(),
        path: if short {
            repo.path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| repo.path.to_string_lossy().to_string())
        } else {
            repo.path.to_string_lossy().to_string()
        },
        remote: repo.remote_url.clone().unwrap_or_else(|| "-".to_string()),
        branch: status.branch,
        sync: sync_label(&status.sync),
        worktree: if status.worktree_dirty {
            "dirty".to_string()
        } else {
            "clean".to_string()
        },
    }
}

fn parse_branch(header: &str) -> String {
    let header = header.strip_prefix("## ").unwrap_or(header);
    let before_tracking = header.split("...").next().unwrap_or(header);
    before_tracking
        .split_whitespace()
        .next()
        .unwrap_or("-")
        .to_string()
}

fn parse_marker(header: &str, marker: &str) -> Option<usize> {
    let marker_pos = header.find(marker)?;
    let rest = &header[marker_pos + marker.len()..];
    let digits: String = rest
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn sync_label(sync: &SyncStatus) -> String {
    match sync {
        SyncStatus::Clean => "clean".to_string(),
        SyncStatus::NeedsPush(n) => format!("ahead {n}"),
        SyncStatus::NeedsPull(n) => format!("behind {n}"),
        SyncStatus::Diverged { ahead, behind } => format!("diverged +{ahead}/-{behind}"),
        SyncStatus::NoUpstream => "no upstream".to_string(),
        SyncStatus::MissingPath => "missing path".to_string(),
        SyncStatus::NotGitRepo => "not a git repo".to_string(),
        SyncStatus::Unknown => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean() {
        let status = RepoStatus::parse("## main...origin/main\n");
        assert_eq!(status.branch, "main");
        assert_eq!(status.sync, SyncStatus::Clean);
        assert!(!status.worktree_dirty);
    }

    #[test]
    fn parses_ahead() {
        let status = RepoStatus::parse("## main...origin/main [ahead 2]\n");
        assert_eq!(status.sync, SyncStatus::NeedsPush(2));
    }

    #[test]
    fn parses_behind() {
        let status = RepoStatus::parse("## main...origin/main [behind 3]\n");
        assert_eq!(status.sync, SyncStatus::NeedsPull(3));
    }

    #[test]
    fn parses_diverged_and_dirty() {
        let status =
            RepoStatus::parse("## main...origin/main [ahead 1, behind 2]\n M src/main.rs\n");
        assert_eq!(
            status.sync,
            SyncStatus::Diverged {
                ahead: 1,
                behind: 2
            }
        );
        assert!(status.worktree_dirty);
    }

    #[test]
    fn parses_no_upstream() {
        let status = RepoStatus::parse("## feature\n");
        assert_eq!(status.sync, SyncStatus::NoUpstream);
    }
}
