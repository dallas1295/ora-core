use crate::domain::LocalNote;
use crate::error::RoughError;
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};

pub struct Index {
    pub(crate) pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct IndexedNote {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
}

impl Index {
    pub async fn new(shelf_path: &Path) -> Result<Self, RoughError> {
        let db_path = shelf_path.join(".shelf.db");
        let connection_path = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&connection_path).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                path TEXT UNIQUE NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS contents USING fts5(title, content, content='notes', content_rowid='id')",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
             INSERT INTO contents(rowid, title, content) VALUES (new.id, new.title, new.content);
            END",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
             INSERT INTO contents(contents, rowid, title, content) VALUES('delete', old.id, old.title, old.content);
            END"
        ).execute(&pool).await?;

        sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
             INSERT INTO contents(contents, rowid, title, content) VALUES('delete', old.id, old.title, old.content);
             INSERT INTO contents(rowid, title, content) VALUES (new.id, new.title, new.content);
            END"
        ).execute(&pool).await?;

        Ok(Index { pool })
    }

    pub async fn index_note(&self, note: &LocalNote) -> Result<(), RoughError> {
        sqlx::query(
            "INSERT OR REPLACE INTO notes (title,content, path, updated_at)
            VALUES (?,?,?, CURRENT_TIMESTAMP)",
        )
        .bind(&note.title)
        .bind(&note.content)
        .bind(note.path.display().to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_note(&self, note: &LocalNote) -> Result<bool, RoughError> {
        let res = sqlx::query("DELETE FROM notes WHERE path = ?")
            .bind(note.path.display().to_string())
            .execute(&self.pool)
            .await?;

        Ok(res.rows_affected() > 0)
    }

    pub async fn get_by_path(&self, path: &Path) -> Result<Option<IndexedNote>, RoughError> {
        let row = sqlx::query("SELECT title, content, path FROM notes WHERE path = ?")
            .bind(path.display().to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let title: String = row.get(0);
                let content: String = row.get(1);
                let path_str: String = row.get(2);

                Ok(Some(IndexedNote {
                    title,
                    content,
                    path: PathBuf::from(path_str),
                }))
            }
            None => Ok(None),
        }
    }
}
