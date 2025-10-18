use crate::domain::LocalNote;
use crate::error::OraError;
use crate::watcher::index::Index;
use std::path::Path;

fn is_markdown_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("md")
}

#[derive(Clone)]
pub struct FileIndexHandler {
    index: Index,
}

impl FileIndexHandler {
    pub fn new(index: Index) -> Self {
        Self { index }
    }

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
                println!("Indexed new note: {}", note.title);
            }
            Err(e) => {
                eprintln!("Failed to open note for indexing: {:?}, error: {}", path, e)
            }
        }
        Ok(())
    }

    pub fn handle_modify(&self, path: &Path) -> Result<(), OraError> {
        if !is_markdown_file(path) {
            return Ok(());
        }
        match LocalNote::open(path) {
            Ok(note) => {
                if !self.index.exists(path)? {
                    self.index.index_note(&note)?;
                    println!("Indexed new note via modify: {}", note.title);
                } else {
                    self.index.index_note(&note)?;
                    println!("Updated indexed note: {}", note.title);
                }
            }
            Err(_) => {
                let deleted_note = LocalNote {
                    title: String::new(),
                    content: String::new(),
                    path: path.to_path_buf(),
                };
                let was_removed = self.index.remove_note(&deleted_note)?;
                if was_removed {
                    println!("Removed missing note: {:?}", path);
                }
            }
        }
        Ok(())
    }

    pub fn handle_remove(&self, path: &Path) -> Result<(), OraError> {
        if !is_markdown_file(path) {
            return Ok(());
        }

        let deleted_note = LocalNote {
            title: String::new(),
            content: String::new(),
            path: path.to_path_buf(),
        };

        let was_removed = self.index.remove_note(&deleted_note)?;
        if was_removed {
            println!("Removed indexed note: {:?}", path);
        }
        Ok(())
    }
}
