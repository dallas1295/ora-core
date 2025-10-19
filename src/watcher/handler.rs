//! File system event handling for the watcher service.
//!
//! This module provides the `FileIndexHandler` which processes file system
//! events and updates the search index accordingly. It handles file creation,
//! modification, and deletion events, filtering for relevant Markdown files.
//!
//! # Event Processing
//!
//! The handler processes three types of events:
//! - **Create**: New files are added to the index
//! - **Modify**: Existing files are updated in the index
//! - **Remove**: Deleted files are removed from the index
//!
//! # File Filtering
//!
//! Only Markdown files (`.md` extension) that are not hidden are processed.
//! Hidden files (starting with `.`) and other file types are ignored.

use crate::domain::LocalNote;
use crate::error::OraError;
use crate::watcher::index::Index;
use std::path::Path;

/// Checks if a file path represents a processable Markdown file.
///
/// Returns `true` only for files with:
/// - `.md` extension
/// - Not hidden (doesn't start with `.`)
///
/// # Arguments
/// * `path` - The file path to check
///
/// # Returns
/// `true` if the file should be processed, `false` otherwise
fn is_markdown_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("md")
        && !path.file_name().unwrap().to_str().unwrap().starts_with('.')
}

/// Handles file system events and maintains the search index.
///
/// The `FileIndexHandler` is responsible for processing debounced file
/// system events and updating the SQLite search index accordingly.
/// It filters events to only process relevant Markdown files.
#[derive(Clone)]
pub struct FileIndexHandler {
    /// The search index to update when processing events.
    index: Index,
}

impl FileIndexHandler {
    /// Creates a new file index handler.
    ///
    /// # Arguments
    /// * `index` - The search index to update when processing events
    ///
    /// # Returns
    /// A new `FileIndexHandler` instance
    pub fn new(index: Index) -> Self {
        Self { index }
    }

    /// Handles file creation events.
    ///
    /// Processes new file creation by adding the file to the search index
    /// if it's a valid Markdown file. Skips files that are already indexed
    /// to avoid duplicates.
    ///
    /// # Arguments
    /// * `path` - Path to the created file
    ///
    /// # Behavior
    /// - Only processes Markdown files (`.md` extension)
    /// - Skips hidden files
    /// - Checks if file is already indexed before adding
    /// - Logs errors for files that can't be opened
    ///
    /// # Errors
    /// Returns `OraError` if indexing operations fail
    pub fn handle_create(&self, path: &Path) -> Result<(), OraError> {
        if !is_markdown_file(path) {
            return Ok(());
        }

        if self.index.exists(path)? {
            return Ok(());
        }

        match LocalNote::open(path) {
            Ok(note) => {
                self.index.index_note(&note)?;
            }
            Err(e) => {
                eprintln!("Failed to open note for indexing: {:?}, error: {}", path, e)
            }
        }
        Ok(())
    }

    /// Handles file modification events.
    ///
    /// Processes file changes by updating the search index. If the file
    /// can't be opened (e.g., it was deleted), it's removed from the index.
    ///
    /// # Arguments
    /// * `path` - Path to the modified file
    ///
    /// # Behavior
    /// - Only processes Markdown files (`.md` extension)
    /// - Skips hidden files
    /// - Updates existing entries in the index
    /// - Removes files from index if they can't be read
    ///
    /// # Errors
    /// Returns `OraError` if indexing operations fail
    pub fn handle_modify(&self, path: &Path) -> Result<(), OraError> {
        if !is_markdown_file(path) {
            return Ok(());
        }
        match LocalNote::open(path) {
            Ok(note) => {
                self.index.index_note(&note)?;
            }
            Err(_) => {
                let deleted_note = LocalNote {
                    title: String::new(),
                    content: String::new(),
                    path: path.to_path_buf(),
                };
                self.index.remove_note(&deleted_note)?;
            }
        }
        Ok(())
    }

    /// Handles file deletion events.
    ///
    /// Processes file deletion by removing the file from the search index.
    /// This ensures that deleted files don't appear in search results.
    ///
    /// # Arguments
    /// * `path` - Path to the deleted file
    ///
    /// # Behavior
    /// - Only processes Markdown files (`.md` extension)
    /// - Skips hidden files
    /// - Removes entries from the search index
    ///
    /// # Errors
    /// Returns `OraError` if the removal operation fails
    pub fn handle_remove(&self, path: &Path) -> Result<(), OraError> {
        if !is_markdown_file(path) {
            return Ok(());
        }

        let deleted_note = LocalNote {
            title: String::new(),
            content: String::new(),
            path: path.to_path_buf(),
        };

        self.index.remove_note(&deleted_note)?;
        Ok(())
    }

    /// Gets access to the underlying search index.
    ///
    /// This method is only available when running with the `test-methods` feature.
    /// It provides direct access to the index for testing purposes.
    ///
    /// # Feature Flag
    ///
    /// This method is only available when compiling with `--features test-methods`.
    ///
    /// # Testing Usage
    ///
    /// In tests, use this method to access the index directly for verification
    /// and to avoid creating conflicting index instances.
    #[cfg(feature = "test-methods")]
    pub fn get_index(&self) -> Index {
        self.index.clone()
    }
}
