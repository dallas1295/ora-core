use crate::domain::LocalNote;
use crate::error::OraError;
use crate::watcher::index::Index;
use std::path::Path;

fn is_markdown_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("md")
        && !path.file_name().unwrap().to_str().unwrap().starts_with('.')
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

    // This method is ONLY available when running `cargo test --features test-methods`.
    // It's useful for accessing the Index instance from the watcher service.
    #[cfg(feature = "test-methods")]
    pub fn get_index(&self) -> Index {
        self.index.clone()
    }
}
