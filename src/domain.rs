use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NoteError {
    #[error("invalid path")]
    InvalidPath,
    #[error("invalid title")]
    InvalidTitle,
    #[error(transparent)]
    FileError(std::io::Error),
}

/// A note stored locally on disk as a Markdown file (`.md`).
pub struct LocalNote {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
}

impl LocalNote {
    /// Creates a new note file on disk.
    ///
    /// Validates the title, builds a Markdown file path under the provided directory,
    /// writes the content to disk, and returns a `LocalNote` representing it.
    ///
    /// Returns `NoteError::InvalidTitle` on a bad title, or `FileError` if writing fails.
    pub fn create(title: String, content: String, path: &Path) -> Result<LocalNote, NoteError> {
        let note_title = Self::valid_title(&title)?;
        let note_path = path.join(format!("{note_title}.md"));

        fs::write(&note_path, &content).map_err(NoteError::FileError)?;

        Ok(LocalNote {
            title: note_title,
            content: content,
            path: note_path,
        })
    }

    /// Reloads this note from disk, returning a new `LocalNote`.
    ///
    /// Useful when the file might have been modified externally (e.g. in Vim, Obsidian, etc.).
    ///
    /// Does **not** mutate `self`; instead returns a refreshed instance holding
    /// the current file contents. Returns `FileError` if reading fails.
    pub fn reload(&self) -> Result<LocalNote, NoteError> {
        let data = fs::read_to_string(&self.path).map_err(NoteError::FileError)?;

        Ok(LocalNote {
            title: self.title.clone(),
            content: data,
            path: self.path.clone(),
        })
    }

    /// Returns an in-memory copy of this note with new content.
    ///
    /// `with_content` does not persist to disk; it only creates a variant of
    /// this `LocalNote` with `content` replaced. Useful for version history
    /// or sync engines.
    pub fn with_content(&self, new_content: String) -> LocalNote {
        LocalNote {
            title: self.title.clone(),
            content: new_content,
            path: self.path.clone(),
        }
    }

    /// Returns an in-memory copy of this note with a new title.
    ///
    /// Produces a `LocalNote` pointing to a new `.md` path but
    /// leaves the filesystem unchanged until `persist_rename` is called.
    ///
    /// Returns `InvalidTitle` if the new title is not valid.
    pub fn with_title(&self, new_title: String) -> Result<LocalNote, NoteError> {
        let new_title = Self::valid_title(&new_title)?;
        let base_dir = self.path.parent().ok_or(NoteError::InvalidPath)?;
        let new_path = base_dir.join(format!("{new_title}.md"));

        Ok(LocalNote {
            title: new_title,
            content: self.content.clone(),
            path: new_path,
        })
    }

    /// Saves the current note to disk at `self.path`.
    ///
    /// Uses an atomic write (tempfile + rename) to avoid corruption.
    /// Overwrites the previous contents of the file.
    pub fn save(&self) -> Result<(), NoteError> {
        Self::write_atomic(&self.path, self.content.as_bytes())
    }

    /// Persists a rename on disk from an old path.
    ///
    /// Typically used after [`with_title`] to commit the updated `path`
    /// of a note by renaming the old file.
    pub fn persist_rename(&self, old_path: &Path) -> Result<(), NoteError> {
        fs::rename(old_path, &self.path).map_err(NoteError::FileError)?;
        Ok(())
    }

    /// Deletes this note from disk.
    pub fn delete(&self) -> Result<(), NoteError> {
        let path = &self.path;
        fs::remove_file(path).map_err(NoteError::FileError)?;
        Ok(())
    }

    /// Validates a proposed note title.
    ///
    /// Trims whitespace, ensures it is not empty, and rejects
    /// OS‑invalid characters (`/`, `\`, `:`, `"`, `*`, `?`, `<`, `>`, `|`).
    fn valid_title(title: &str) -> Result<String, NoteError> {
        let trimmed = title.trim();

        if trimmed.is_empty() {
            return Err(NoteError::InvalidTitle);
        }

        if trimmed.contains(&['/', '\\', ':', '"', '*', '?', '<', '>', '|'][..]) {
            return Err(NoteError::InvalidTitle);
        }

        Ok(trimmed.to_owned())
    }

    /// Internal helper to atomically write note content to disk.
    ///
    /// Writes data to a temporary file in the target directory
    /// and renames it in place, guaranteeing the note file is never
    /// left in a corrupted state if a crash occurs mid-write.
    fn write_atomic(path: &Path, data: &[u8]) -> Result<(), NoteError> {
        let dir = path.parent().ok_or(NoteError::InvalidPath)?;
        let mut tmp = NamedTempFile::new_in(dir).map_err(NoteError::FileError)?;
        tmp.write_all(data).map_err(NoteError::FileError)?;
        tmp.persist(path)
            .map_err(|e| NoteError::FileError(e.error))?;
        Ok(())
    }
}
