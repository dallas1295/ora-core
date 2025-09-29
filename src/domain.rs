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
#[derive(Clone)]
pub struct LocalNote {
    pub title: String,
    pub content: String,
    pub path: PathBuf,
}

impl LocalNote {
    ///
    /// Ensures the content begins with a Markdown `# Title` heading,
    /// extracts the human‑readable title from that heading,
    /// derives a slugified filename from the title,
    /// and writes the note to disk at the given path.
    ///
    /// Returns a `LocalNote` with the parsed title, full content,
    /// and safe slugified path.
    /// Returns `NoteError` if the title is invalid or if writing fails.
    pub fn create(title: &str, content: &str, path: &Path) -> Result<LocalNote, NoteError> {
        let full_content = if content.starts_with('#') {
            content.to_string()
        } else {
            format!("# {}\n\n{}", title.trim(), content)
        };

        let note_title = sanitize_title(&full_content);
        let slug = slugify_title(&note_title);
        let filename = is_unique(&slug, &path);
        let note_path = path.join(filename);

        fs::write(&note_path, &full_content).map_err(NoteError::FileError)?;

        Ok(LocalNote {
            title: note_title,
            content: full_content,
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
        let note_title = sanitize_title(&data);

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

    /// Returns a new `LocalNote` with its title updated,
    /// including replacing the first H1 heading in the content
    /// and updating the slugified file path.
    ///
    /// Creates a fresh copy of the note in memory; the filesystem
    /// is not modified until `persist_rename` is called.
    ///
    /// If the content has no existing heading, a new one is inserted.
    /// Returns `NoteError` if the title cannot be processed.
    pub fn with_title(&self, new_title: &str) -> Result<LocalNote, NoteError> {
        let mut new_content = String::new();
        let mut lines = self.content.lines();
        lines.next(); // skip old first line
        new_content.push_str(&format!("# {}", new_title));
        for line in lines {
            new_content.push('\n');
            new_content.push_str(line);
        }

        let title = sanitize_title(&new_content);
        let slug = slugify_title(&title);
        let base_dir = self.path.parent().ok_or(NoteError::InvalidPath)?;
        let filename = is_unique(&slug, &base_dir);
        let new_path = base_dir.join(filename);

        Ok(LocalNote {
            title: title,
            content: new_content,
            path: new_path,
        })
    }
    /// Saves the current note to disk at `self.path`.
    ///
    /// Uses an atomic write (tempfile + rename) to avoid corruption.
    /// Overwrites the previous contents of the file.
    pub fn save(&self) -> Result<(), NoteError> {
        write_atomic(&self.path, self.content.as_bytes())
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

    pub fn open(path: &Path) -> Result<LocalNote, NoteError> {
        let content = fs::read_to_string(path).map_err(NoteError::FileError)?;

        let title = sanitize_title(&content);

        Ok(LocalNote {
            title,
            content,
            path: path.to_path_buf(),
        })
    }
}

fn is_unique(base: &str, dir: &Path) -> PathBuf {
    let mut count = 0;
    loop {
        let candidate = if count == 0 {
            dir.join(format!("{}.md", base))
        } else {
            dir.join(format!("{}_{}.md", base, count))
        };

        if !candidate.exists() {
            return candidate;
        }
        count += 1;
    }
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

/// Converts a human‑readable title into a filesystem‑safe slug.
///
/// Lowercases all characters, replaces whitespace and hyphens with `_`,
/// strips unsafe characters, and appends a `.md` extension to produce
/// a valid Markdown filename.
///
/// Example: `"Meeting Notes!"` → `"meeting_notes.md"`.
fn slugify_title(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' {
                '_'
            } else if c == '_' {
                c
            } else {
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect();

    slug.trim_matches('_').to_owned()
}

/// Extracts a human‑readable title from the first line of note content.
///
/// Looks for a Markdown H1 heading (`# Title`) on the first line,
/// strips the `#` marker and leading/trailing whitespace, and returns
/// the cleaned title string.
///
/// If the first line is missing or empty, returns `"Untitled"`.
fn sanitize_title(content: &str) -> String {
    let first_line = content.lines().next();

    match first_line {
        Some(line) => {
            let forbidden = ['/', '\\', ':', '"', '*', '?', '<', '>', '|'];
            let title: String = line
                .chars()
                .skip_while(|&c| c == '#')
                .skip_while(|&c| c.is_whitespace())
                .map(|c| if forbidden.contains(&c) { '_' } else { c })
                .collect::<String>()
                .trim_end()
                .to_string();

            if title.is_empty() {
                "Untitled".to_string()
            } else {
                title
            }
        }
        None => "Untitled".to_string(),
    }
}
