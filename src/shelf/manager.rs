use crate::domain::LocalNote;
use crate::error::OraError;
use crate::shelf::storage::Shelf;
use std::fs;

/// A manager providing high‑level operations for notes inside a single [`Shelf`].
///
/// Wraps a reference to a [`Shelf`] and exposes helper methods for creating,
/// reading, listing, deleting, and updating [`LocalNote`]s within that shelf.
pub struct ShelfManager<'a> {
    shelf: &'a Shelf,
}

impl<'a> ShelfManager<'a> {
    /// Creates a new manager for the given [`Shelf`].
    pub fn new(shelf: &'a Shelf) -> Self {
        ShelfManager { shelf }
    }

    /// Returns the name of the managed shelf.
    pub fn shelf_name(&self) -> &str {
        &self.shelf.name
    }

    /// Retrieves a note by its slug (filename without `.md`).
    ///
    /// Constructs the path `{shelf_root}/{slug}.md` and attempts to open it.
    ///
    /// # Errors
    /// Returns [`OraError`] if the note cannot be read or parsed.
    pub fn get_note(&self, slug: &str) -> Result<LocalNote, OraError> {
        let note_path = self.shelf.root.join(format!("{slug}.md"));
        Ok(LocalNote::open(&note_path)?)
    }

    /// Lists all notes in the shelf.
    ///
    /// Scans the shelf directory for `*.md` files, deserializes each into a
    /// [`LocalNote`], and returns them as a vector.
    ///
    /// # Errors
    /// Returns [`OraError`] if the directory or any note file cannot be read.
    pub fn list_notes(&self) -> Result<Vec<LocalNote>, OraError> {
        let mut notes = Vec::new();
        for entry in fs::read_dir(&self.shelf.root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                let note = LocalNote::open(&path)?; // auto NoteError -> OraError
                notes.push(note);
            }
        }
        Ok(notes)
    }

    /// Creates a new note inside the shelf.
    ///
    /// Uses the given `title` and `content`. Slugifies the title and chooses
    /// a unique filename under the shelf root.
    ///
    /// # Errors
    /// Returns [`OraError`] if the note cannot be created on disk.
    pub fn create_note(&self, title: &str, content: &str) -> Result<LocalNote, OraError> {
        Ok(LocalNote::create(title, content, &self.shelf.root)?)
    }

    /// Deletes a note in the shelf by slug.
    ///
    /// Constructs `{shelf_root}/{slug}.md`, then removes it from disk.
    ///
    /// # Errors
    /// Returns [`OraError`] if the filesystem operation fails.
    pub fn delete_note(&self, slug: &str) -> Result<(), OraError> {
        let note_path = self.shelf.root.join(format!("{slug}.md"));

        let note_to_delete = LocalNote {
            title: String::new(),
            content: String::new(),
            path: note_path,
        };

        note_to_delete.delete()?; // NoteError -> OraError
        Ok(())
    }

    /// Updates an existing note in the shelf.
    ///
    /// - If `new_content` is set, replaces the note's content.  
    /// - If `new_title` is set, updates the first‑line heading and slugified filename.  
    /// - Saves the modified note to disk, replacing or renaming the old file as needed.  
    ///
    /// # Errors
    /// Returns [`OraError`] if reading, writing, or deleting underlying files fails.
    pub fn update_note(
        &self,
        slug: &str,
        new_title: Option<&str>,
        new_content: Option<&str>,
    ) -> Result<LocalNote, OraError> {
        let original_note = self.get_note(slug)?;
        let mut final_note = original_note.clone();

        if let Some(title) = new_title {
            final_note = final_note.with_title(title)?;
        };

        if let Some(content) = new_content {
            final_note = final_note.with_content(content);
        }

        if final_note.path != original_note.path {
            final_note.save()?;
            fs::remove_file(&original_note.path)?;
        } else {
            final_note.save()?;
        }

        final_note.reload()?;

        Ok(final_note)
    }
}
