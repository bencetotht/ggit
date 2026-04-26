use crate::db::Repository;
use crate::status::StatusRow;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

pub fn repo_table(repos: &[Repository]) -> String {
    let mut table = table();
    table.set_header(vec!["name", "remote", "path"]);
    for repo in repos {
        table.add_row(vec![
            repo.name.clone(),
            repo.remote_url
                .clone()
                .unwrap_or_else(|| "no remote configured".to_string()),
            repo.path.to_string_lossy().to_string(),
        ]);
    }
    table.to_string()
}

pub fn stored_repo_table(repos: &[Repository]) -> String {
    let mut table = table();
    table.set_header(vec!["name", "branch", "remote", "path"]);
    for repo in repos {
        table.add_row(vec![
            repo.name.clone(),
            repo.current_branch
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            repo.remote_url.clone().unwrap_or_else(|| "-".to_string()),
            repo.path.to_string_lossy().to_string(),
        ]);
    }
    table.to_string()
}

pub fn status_table(rows: &[StatusRow], short: bool) -> String {
    let mut table = table();
    if short {
        table.set_header(vec!["name", "branch", "sync", "worktree"]);
        for row in rows {
            table.add_row(vec![
                row.name.clone(),
                row.branch.clone(),
                row.sync.clone(),
                row.worktree.clone(),
            ]);
        }
    } else {
        table.set_header(vec!["name", "branch", "sync", "worktree", "remote", "path"]);
        for row in rows {
            table.add_row(vec![
                row.name.clone(),
                row.branch.clone(),
                row.sync.clone(),
                row.worktree.clone(),
                row.remote.clone(),
                row.path.clone(),
            ]);
        }
    }
    table.to_string()
}

fn table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}
