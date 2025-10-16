use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NoteError {
    /// Returned when a filesystem path cannot be resolved.
    #[error("invalid path")]
    InvalidPath,

    /// Returned when a note is saved, but there are no changes to it.
    #[error("no changes to file")]
    NoChanges,

    /// Wraps any underlying I/O error (read/write/rename/delete).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A note stored locally on disk as a Markdown file (`.md`).
#[derive(Clone)]
pub struct LocalNote {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
}

impl LocalNote {
    /// Creates a new note with the given title and content.
    /// Uses the title as the filename (with .md extension) and writes
    /// the content exactly as provided to disk.
    ///
    /// Empty titles are replaced with "Untitled". If a file with the same
    /// name exists, a number suffix is added (e.g., "My Note 1.md").
    ///
    /// Returns a `LocalNote` with the title, content, and file path.
    ///
    /// # Errors
    /// - [`NoteError::InvalidPath`] if the target path cannot be determined
    /// - [`NoteError::Io`] if the file cannot be written
    pub fn create(title: &str, content: &str, path: &Path) -> Result<LocalNote, NoteError> {
        let note_title = if title.trim().is_empty() {
            "Untitled".to_string()
        } else {
            title.trim().to_string()
        };

        let filename = create_unique_filename(&note_title, &path);
        let note_path = path.join(filename);

        fs::write(&note_path, content)?;

        Ok(LocalNote {
            title: note_title,
            content: content.to_string(),
            path: note_path,
        })
    }

    /// Reloads this note from disk, returning a new `LocalNote`.
    ///
    /// Useful when the file might have been modified externally (e.g. in Vim, Obsidian, etc.).
    ///
    /// Does **not** mutate `self`; instead returns a refreshed instance holding
    /// the current file contents. The title is extracted from the filename.
    ///
    /// # Errors
    /// - [`NoteError::Io`] if the file cannot be read
    pub fn reload(&self) -> Result<LocalNote, NoteError> {
        let data = fs::read_to_string(&self.path)?;
        let note_title = extract_title_from_path(&self.path);

        Ok(LocalNote {
            title: note_title,
            content: data,
            path: self.path.clone(),
        })
    }

    /// Returns an in-memory copy of this note with new content.
    ///
    /// `with_content` does not persist to disk; it only creates a variant of
    /// this `LocalNote` with `content` replaced. Useful for version history
    /// or sync engines.
    pub fn with_content(&self, new_content: &str) -> LocalNote {
        LocalNote {
            title: self.title.clone(),
            content: new_content.to_string(),
            path: self.path.clone(),
        }
    }

    /// Saves the note with a new title, handling both title change and content changes atomically.
    ///
    /// This method will:
    /// 1. Create a new filename based on the title
    /// 2. Atomically write the current content to the new path
    /// 3. Remove the old file if the path changed
    /// 4. Update the note's title and path in memory
    ///
    /// Empty titles are replaced with "Untitled". If a file with the same
    /// name exists, a number suffix is added (e.g., "My Note 1.md").
    ///
    /// # Errors
    /// - [`NoteError::InvalidPath`] if the parent directory cannot be determined
    /// - [`NoteError::NoChanges`] if there are no changes to the file
    /// - [`NoteError::Io`] if write or rename fails
    pub fn save_as(&mut self, title: &str) -> Result<(), NoteError> {
        let new_title = if title.trim().is_empty() {
            "Untitled".to_string()
        } else {
            title.trim().to_string()
        };

        if new_title == self.title {
            return self.save();
        }

        let base_dir = self.path.parent().ok_or(NoteError::InvalidPath)?;
        let new_filename = create_unique_filename(&new_title, base_dir);
        let new_path = base_dir.join(new_filename);

        if self.path.exists() {
            if let Ok(existing_content) = fs::read_to_string(&self.path) {
                if existing_content == self.content && new_path == self.path {
                    return Err(NoteError::NoChanges);
                }
            }
        }

        write_atomic(&new_path, self.content.as_bytes())?;

        if new_path != self.path {
            fs::remove_file(&self.path)?;
        }

        self.title = new_title;
        self.path = new_path;

        Ok(())
    }

    /// Saves the current note to disk at `self.path`.
    ///
    /// Uses an atomic write (tempfile + rename) to avoid corruption.
    /// Overwrites the previous contents of the file.
    ///
    /// # Errors
    /// - [`NoteError::InvalidPath`] if the parent directory cannot be determined
    /// - [`NoteError::NoChanges`] if there are no changes to the file
    /// - [`NoteError::Io`] if write or rename fails
    pub fn save(&self) -> Result<(), NoteError> {
        if self.path.exists() {
            if let Ok(existing_content) = fs::read_to_string(&self.path) {
                if existing_content == self.content {
                    return Err(NoteError::NoChanges);
                }
            }
        }
        write_atomic(&self.path, self.content.as_bytes())
    }



    /// Deletes this note from disk.
    ///
    /// # Errors
    /// - [`NoteError::Io`] if the file cannot be removed
    pub fn delete(&self) -> Result<(), NoteError> {
        fs::remove_file(&self.path)?;
        Ok(())
    }

    /// Opens an existing note from disk at the given `path`.
    ///
    /// Reads the file contents into memory and extracts the title
    /// from the filename (without .md extension), returning a new `LocalNote`.
    ///
    /// # Errors
    /// - [`NoteError::Io`] if reading fails
    pub fn open(path: &Path) -> Result<LocalNote, NoteError> {
        let content = fs::read_to_string(path)?;
        let title = extract_title_from_path(path);

        Ok(LocalNote {
            title,
            content,
            path: path.to_path_buf(),
        })
    }
}

/// Checks for a unique Markdown filename in `dir` based on the title.
///
/// If `title.md` exists, tries `title 1.md`, `title 2.md`, ... until a free
/// path is found. Returns the first non‑existing candidate `PathBuf`.
fn create_unique_filename(title: &str, dir: &Path) -> PathBuf {
    let mut count = 0;
    loop {
        let candidate = if count == 0 {
            dir.join(format!("{}.md", title))
        } else {
            dir.join(format!("{} {}.md", title, count))
        };

        if !candidate.exists() {
            return candidate;
        }
        count += 1;
    }
}

/// Extracts the title from a file path by removing the .md extension.
///
/// If the filename is empty or doesn't have a .md extension, returns "Untitled".
fn extract_title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Untitled")
        .trim()
        .to_string()
}

/// Internal helper to atomically write note content to disk.
///
/// Writes data to a temporary file in the target directory and renames it
/// in place, guaranteeing the note file is never left in a corrupted state.
///
/// # Errors
/// - [`NoteError::InvalidPath`] if the parent directory cannot be determined
/// - [`NoteError::Io`] if writing or persisting the tempfile fails
fn write_atomic(path: &Path, data: &[u8]) -> Result<(), NoteError> {
    let dir = path.parent().ok_or(NoteError::InvalidPath)?;
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(data)?;
    tmp.persist(path).map_err(|e| NoteError::Io(e.error))?;
    Ok(())
}
