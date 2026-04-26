use crate::config;
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Rows};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    pub id: Option<i64>,
    pub name: String,
    pub path: PathBuf,
    pub remote_url: Option<String>,
    pub current_branch: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        config::ensure_parent(path)?;
        let conn = Connection::open(path)
            .with_context(|| format!("could not open database {}", path.display()))?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS repositories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                remote_url TEXT,
                current_branch TEXT,
                last_seen_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_repositories_name
            ON repositories(name);

            CREATE INDEX IF NOT EXISTS idx_repositories_remote_url
            ON repositories(remote_url);
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_repository(&self, repo: &Repository) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO repositories
                (name, path, remote_url, current_branch, last_seen_at, created_at, updated_at)
            VALUES
                (?1, ?2, ?3, ?4, ?5, ?5, ?5)
            ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                remote_url = excluded.remote_url,
                current_branch = excluded.current_branch,
                last_seen_at = excluded.last_seen_at,
                updated_at = excluded.updated_at
            "#,
            params![
                repo.name,
                repo.path.to_string_lossy(),
                repo.remote_url,
                repo.current_branch,
                now
            ],
        )?;
        Ok(())
    }

    pub fn list_repositories(&self) -> Result<Vec<Repository>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, remote_url, current_branch FROM repositories ORDER BY name, path",
        )?;
        let mut rows = stmt.query([])?;
        collect_rows(&mut rows)
    }

    pub fn filter_by_name(&self, filter: &str) -> Result<Vec<Repository>> {
        let needle = format!("%{}%", filter.to_lowercase());
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, path, remote_url, current_branch
            FROM repositories
            WHERE lower(name) LIKE ?1
            ORDER BY name, path
            "#,
        )?;
        let mut rows = stmt.query(params![needle])?;
        collect_rows(&mut rows)
    }

    pub fn find_by_path(&self, path: &Path) -> Result<Option<Repository>> {
        self.conn
            .query_row(
                "SELECT id, name, path, remote_url, current_branch FROM repositories WHERE path = ?1",
                params![path.to_string_lossy()],
                row_to_repository,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn remove_by_path(&self, path: &Path) -> Result<usize> {
        self.conn
            .execute(
                "DELETE FROM repositories WHERE path = ?1",
                params![path.to_string_lossy()],
            )
            .map_err(Into::into)
    }

    pub fn clear(&self) -> Result<usize> {
        self.conn
            .execute("DELETE FROM repositories", [])
            .map_err(Into::into)
    }
}

fn row_to_repository(row: &rusqlite::Row<'_>) -> rusqlite::Result<Repository> {
    let path: String = row.get(2)?;
    Ok(Repository {
        id: row.get(0)?,
        name: row.get(1)?,
        path: PathBuf::from(path),
        remote_url: row.get(3)?,
        current_branch: row.get(4)?,
    })
}

fn collect_rows(rows: &mut Rows<'_>) -> Result<Vec<Repository>> {
    let mut repos = Vec::new();
    while let Some(row) = rows.next()? {
        repos.push(row_to_repository(row)?);
    }
    Ok(repos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn repo(path: PathBuf, remote: Option<&str>) -> Repository {
        Repository {
            id: None,
            name: "example".to_string(),
            path,
            remote_url: remote.map(str::to_string),
            current_branch: Some("main".to_string()),
        }
    }

    #[test]
    fn upserts_and_filters_repository() {
        let dir = tempdir().unwrap();
        let db = Database::open(&dir.path().join("db.sqlite")).unwrap();
        let path = dir.path().join("repo");

        db.upsert_repository(&repo(path.clone(), Some("old")))
            .unwrap();
        db.upsert_repository(&repo(path.clone(), Some("new")))
            .unwrap();

        let repos = db.list_repositories().unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].remote_url.as_deref(), Some("new"));
        assert_eq!(db.filter_by_name("EXAM").unwrap().len(), 1);

        db.remove_by_path(&path).unwrap();
        assert!(db.list_repositories().unwrap().is_empty());
    }
}
