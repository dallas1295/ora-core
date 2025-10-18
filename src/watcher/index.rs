use crate::domain::LocalNote;
use crate::error::OraError;
use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Index {
    pub conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct IndexedNote {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
}

impl Index {
    pub fn new(shelf_path: &Path) -> Result<Self, OraError> {
        let db_path = shelf_path.join(".shelf.db");
        let conn = Connection::open(&db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                path TEXT UNIQUE NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS contents USING fts5(title, content, content='notes', content_rowid='id')",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
             INSERT INTO contents(rowid, title, content) VALUES (new.id, new.title, new.content);
            END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
             INSERT INTO contents(contents, rowid, title, content) VALUES('delete', old.id, old.title, old.content);
            END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
             INSERT INTO contents(contents, rowid, title, content) VALUES('delete', old.id, old.title, old.content);
             INSERT INTO contents(rowid, title, content) VALUES (new.id, new.title, new.content);
            END",
            [],
        )?;

        let index = Index {
            conn: Arc::new(Mutex::new(conn)),
        };

        index.index_existing_files(shelf_path)?;

        return Ok(index);
    }

    pub fn index_existing_files(&self, shelf_path: &Path) -> Result<(), OraError> {
        for entry in fs::read_dir(shelf_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.index_existing_files(&path)?;
            } else if let Some(ext) = path.extension() {
                if ext == "md" && !path.file_name().unwrap().to_str().unwrap().starts_with('.') {
                    // Check if file is already indexed to avoid duplicates
                    if !self.exists(&path)? {
                        if let Ok(note) = LocalNote::open(&path) {
                            self.index_note(&note)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn index_note(&self, note: &LocalNote) -> Result<(), OraError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO notes (title, content, path, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
            params![&note.title, &note.content, note.path.display().to_string()],
        )?;
        Ok(())
    }

    pub fn remove_note(&self, note: &LocalNote) -> Result<bool, OraError> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM notes WHERE path = ?",
            params![note.path.display().to_string()],
        )?;
        Ok(rows_affected > 0)
    }

    pub fn exists(&self, path: &Path) -> Result<bool, OraError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM notes WHERE path = ?")?;
        let count: i64 = stmt.query_row(params![path.display().to_string()], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn get_by_path(&self, path: &Path) -> Result<Option<IndexedNote>, OraError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT title, content, path FROM notes WHERE path = ?")?;

        let result = stmt.query_row(params![path.display().to_string()], |row| {
            Ok(IndexedNote {
                title: row.get(0)?,
                content: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
            })
        });

        match result {
            Ok(note) => Ok(Some(note)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(OraError::Other(e.to_string())),
        }
    }
}
