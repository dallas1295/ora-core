use crate::domain::{LocalNote, NoteError};
use crate::shelf::storage::Shelf;
use std::fs;
use std::io;

pub struct ShelfManager<'a> {
    shelf: &'a Shelf,
}

impl<'a> ShelfManager<'a> {
    pub fn new(shelf: &'a Shelf) -> Self {
        ShelfManager { shelf }
    }

    pub fn shelf_name(&self) -> &str {
        &self.shelf.name
    }

    pub fn get_note(&self, slug: &str) -> Result<LocalNote, NoteError> {
        let note_path = self.shelf.root.join(format!("{slug}.md"));
        LocalNote::open(&note_path)
    }

    pub fn list_notes(&self) -> io::Result<Vec<LocalNote>> {
        let mut notes = Vec::new();
        let shelf_path = fs::read_dir(&self.shelf.root)?;

        for entry in shelf_path {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                let note =
                    LocalNote::open(&path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                notes.push(note);
            }
        }
        Ok(notes)
    }

    pub fn create_note(&self, title: &str, content: &str) -> Result<LocalNote, NoteError> {
        LocalNote::create(title, content, &self.shelf.root)
    }

    pub fn delete_note(&self, slug: &str) -> Result<(), NoteError> {
        let note_path = self.shelf.root.join(format!("{slug}.md"));

        let note_to_delete = LocalNote {
            title: String::new(),
            content: String::new(),
            path: note_path,
        };

        note_to_delete.delete()
    }

    pub fn update_note(
        &self,
        slug: &str,
        new_title: Option<&str>,
        new_content: Option<&str>,
    ) -> Result<LocalNote, NoteError> {
        let original_note = self.get_note(slug)?;
        let note_with_new_content = match new_content {
            Some(content) => original_note.with_content(content),
            None => original_note.clone(),
        };

        let final_note = match new_title {
            Some(title) => note_with_new_content.with_title(title)?,
            None => note_with_new_content,
        };

        final_note.save()?;

        if final_note.path != original_note.path {
            fs::remove_file(&original_note.path).map_err(NoteError::FileError)?;
        }

        Ok(final_note)
    }
}
