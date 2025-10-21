//! SQLite-based indexing system for full-text search.
//!
//! This module provides the core indexing functionality that powers the search
//! system. It uses SQLite with FTS5 (Full-Text Search) extension to maintain
//! an up-to-date index of all notes in a shelf.
//!
//! # Database Schema
//!
//! The index creates two main tables:
//! - `notes` - Stores note metadata and content
//! - `contents` - FTS5 virtual table for full-text search
//!
//! # Triggers
//!
//! Automatic triggers keep the FTS5 table synchronized with the notes table:
//! - `notes_ai` - Inserts new notes into search index
//! - `notes_ad` - Removes deleted notes from search index  
//! - `notes_au` - Updates modified notes in search index
//!
//! # Thread Safety
//!
//! The index uses an `Arc<Mutex<Connection>>` to provide thread-safe access
//! to the SQLite database, allowing concurrent read and write operations.

use crate::domain::LocalNote;
use crate::error::OraError;
use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Thread-safe SQLite index for note search functionality.
///
/// The `Index` provides a high-level interface for managing notes in the
/// search database. It handles automatic schema creation, trigger setup,
/// and provides methods for indexing, searching, and managing notes.
///
/// # Database Location
///
/// The index database is stored as `.shelf.db` in the root of the shelf
/// directory. This file contains both the note metadata and the FTS5
/// search index.
///
/// # Concurrency
///
/// The index is designed to be safely used from multiple threads through
/// the use of an `Arc<Mutex<Connection>>`. All operations acquire the
/// mutex lock automatically.
#[derive(Clone)]
pub struct Index {
    /// Shared SQLite connection wrapped in a mutex for thread safety.
    pub conn: Arc<Mutex<Connection>>,
}

/// A note that has been indexed for search.
///
/// Represents a note as stored in the search index, containing the
/// essential information needed for search results and display.
#[derive(Debug, Clone)]
pub struct IndexedNote {
    /// The title of the note (extracted from filename).
    pub title: String,

    /// The full content of the note.
    pub content: String,

    /// The file path where the note is stored.
    pub path: PathBuf,
}

impl Index {
    /// Creates a new search index for the given shelf path.
    ///
    /// Initializes the SQLite database, creates the necessary tables and
    /// triggers, and indexes any existing notes in the shelf directory.
    ///
    /// # Database Setup
    ///
    /// Creates the following schema:
    /// - `notes` table with id, title, content, path, and timestamps
    /// - `contents` FTS5 virtual table for full-text search
    /// - Triggers to keep FTS5 table synchronized
    ///
    /// # Arguments
    /// * `shelf_path` - Path to the shelf directory containing notes
    ///
    /// # Returns
    /// A new `Index` instance ready for use
    ///
    /// # Errors
    /// Returns `OraError` if database creation or initialization fails
    ///
    /// # Side Effects
    /// - Creates `.shelf.db` file in the shelf directory
    /// - Scans and indexes all existing `.md` files recursively
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

    /// Recursively indexes all existing Markdown files in the shelf.
    ///
    /// Scans the shelf directory and all subdirectories for `.md` files,
    /// indexing any that haven't been indexed yet. Hidden files (starting
    /// with `.`) are ignored.
    ///
    /// # Arguments
    /// * `shelf_path` - Root path of the shelf to scan
    ///
    /// # Behavior
    /// - Recursively walks through all subdirectories
    /// - Only processes files with `.md` extension
    /// - Skips hidden files and directories
    /// - Avoids re-indexing files that already exist in the database
    ///
    /// # Errors
    /// Returns `OraError` if directory scanning or file indexing fails
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

    /// Adds or updates a note in the search index.
    ///
    /// Uses `INSERT OR REPLACE` to either create a new entry or update
    /// an existing one. The note is identified by its file path, so
    /// moving a file to a new path will create a separate entry.
    ///
    /// # Arguments
    /// * `note` - The note to index
    ///
    /// # Behavior
    /// - Updates the `updated_at` timestamp automatically
    /// - Triggers FTS5 index update through database triggers
    /// - Thread-safe through mutex locking
    ///
    /// # Errors
    /// Returns `OraError` if the database operation fails
    pub fn index_note(&self, note: &LocalNote) -> Result<(), OraError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO notes (title, content, path, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
            params![&note.title, &note.content, note.path.display().to_string()],
        )?;
        Ok(())
    }

    /// Removes a note from the search index.
    ///
    /// Deletes the note from the database based on its file path.
    /// The FTS5 index is automatically updated through triggers.
    ///
    /// # Arguments
    /// * `note` - The note to remove (only the path is used)
    ///
    /// # Returns
    /// `true` if a note was removed, `false` if no note existed at that path
    ///
    /// # Behavior
    /// - Uses the note's file path as the unique identifier
    /// - Triggers FTS5 index cleanup through database triggers
    /// - Thread-safe through mutex locking
    ///
    /// # Errors
    /// Returns `OraError` if the database operation fails
    pub fn remove_note(&self, note: &LocalNote) -> Result<bool, OraError> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM notes WHERE path = ?",
            params![note.path.display().to_string()],
        )?;
        Ok(rows_affected > 0)
    }

    /// Checks if a note exists in the search index.
    ///
    /// Queries the database to determine if a note with the given
    /// file path has been indexed.
    ///
    /// # Arguments
    /// * `path` - The file path to check
    ///
    /// # Returns
    /// `true` if the note exists in the index, `false` otherwise
    ///
    /// # Errors
    /// Returns `OraError` if the database query fails
    pub fn exists(&self, path: &Path) -> Result<bool, OraError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM notes WHERE path = ?")?;
        let count: i64 = stmt.query_row(params![path.display().to_string()], |row| row.get(0))?;
        Ok(count > 0)
    }

    /// Retrieves a note from the index by its file path.
    ///
    /// Queries the database for a note with the exact file path
    /// and returns it as an `IndexedNote` if found.
    ///
    /// # Arguments
    /// * `path` - The file path of the note to retrieve
    ///
    /// # Returns
    /// `Some(IndexedNote)` if the note exists, `None` otherwise
    ///
    /// # Errors
    /// Returns `OraError` if the database query fails
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
