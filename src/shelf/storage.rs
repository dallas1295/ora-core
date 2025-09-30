use dirs;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShelfError {
    #[error("shelf not found: {0}")]
    NotFound(String),

    #[error("shelf already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid shelf name")]
    InvalidInput,

    #[error("permission denied")]
    PermissionDenied,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Shelf {
    pub root: PathBuf,
    pub name: String,
}

impl Shelf {
    /// Creates a brand-new shelf directory under `~/Documents/shelves/{name}`.
    ///
    /// - Validates the shelf name.
    /// - Returns [`ShelfError::AlreadyExists`] if the directory already exists.
    /// - Returns [`ShelfError::InvalidInput`] if the name is empty or has invalid characters.
    /// - Returns [`ShelfError::Io`] for any underlying filesystem error.

    pub fn new(name: &str) -> Result<Self, ShelfError> {
        let shelf_name = Self::valid_shelf(name)?;
        let root = Shelf::shelf_path(Some(&shelf_name))?;

        if root.exists() {
            return Err(ShelfError::AlreadyExists(shelf_name));
        }

        fs::create_dir_all(&root)?; // converted to ShelfError::Io automatically

        Ok(Self {
            root,
            name: shelf_name,
        })
    }

    /// Opens an existing shelf directory under `~/Documents/shelves/{name}`.
    ///
    /// - Returns [`ShelfError::NotFound`] if the shelf does not exist.
    /// - Returns [`ShelfError::InvalidInput`] if the path exists but is not a directory.
    /// - Returns [`ShelfError::Io`] for other filesystem errors.
    pub fn open(name: &str) -> Result<Self, ShelfError> {
        let root = Self::shelf_path(Some(name))?;

        if !root.exists() {
            return Err(ShelfError::NotFound(name.to_string()));
        }

        if !root.is_dir() {
            return Err(ShelfError::InvalidInput);
        }

        Ok(Self {
            root,
            name: name.to_string(),
        })
    }
    /// Lists all shelves under `~/Documents/shelves`.
    ///
    /// Reads the `shelves` base directory and collects all subdirectories
    /// as shelf names.  
    ///
    /// - Returns a vector of valid shelf names.  
    /// - Returns [`ShelfError::Io`] if the directory cannot be read.  
    /// - Returns [`ShelfError::NotFound`] if the base documents directory is missing.
    pub fn list_shelves() -> Result<Vec<String>, ShelfError> {
        let shelf_base = Shelf::shelf_path(None)?;

        let names = fs::read_dir(&shelf_base)?
            .filter_map(|res| {
                let entry = res.ok()?;
                let ft = entry.file_type().ok()?;
                if ft.is_dir() {
                    Some(entry.file_name().to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .collect();

        Ok(names)
    }

    /// Ensures that the shelf with the given `name` exists.
    ///
    /// If missing, creates a new shelf directory;  
    /// if it already exists, simply opens it.  
    ///
    /// # Errors
    /// - [`ShelfError::InvalidInput`] if the name is empty or has invalid characters
    /// - [`ShelfError::Io`] if the directory creation fails
    pub fn ensure_exists(name: &str) -> Result<Self, ShelfError> {
        let root = Shelf::shelf_path(Some(name))?;
        if !root.exists() {
            fs::create_dir_all(&root)?; // auto Io -> ShelfError
        }

        Shelf::open(name)
    }

    /// Renames this shelf on disk to a new name.
    ///
    /// Updates both the directory on the filesystem and the in‑memory
    /// `name` and `root` fields if successful.
    ///
    /// # Errors
    /// - [`ShelfError::InvalidInput`] if the new name is invalid
    /// - [`ShelfError::AlreadyExists`] if a shelf with the target name already exists
    /// - [`ShelfError::Io`] if the rename operation fails
    pub fn rename(&mut self, new_name: &str) -> Result<(), ShelfError> {
        let valid_new_name = Shelf::valid_shelf(new_name)?;
        let new_path = Self::shelf_path(Some(&valid_new_name))?;

        if new_path.exists() {
            return Err(ShelfError::AlreadyExists(valid_new_name));
        }

        fs::rename(&self.root, &new_path)?; // propagates Io

        self.name = valid_new_name;
        self.root = new_path;

        Ok(())
    }

    /// Deletes this shelf and all its contents from disk.
    ///
    /// Permanently removes the directory at `self.root`.
    /// Returns an error if removal fails (e.g. permissions, in use).
    pub fn delete_shelf(&self) -> Result<(), ShelfError> {
        fs::remove_dir_all(&self.root)?;
        Ok(())
    }

    /// Resolves a given shelf name into a full path under `~/Documents/shelves/{name}`.
    ///
    /// # Errors
    /// - [`ShelfError::NotFound`] if the user's documents directory cannot be determined
    fn shelf_path(name: Option<&str>) -> Result<PathBuf, ShelfError> {
        let docs = dirs::document_dir()
            .ok_or_else(|| ShelfError::NotFound("documents directory".into()))?;

        let shelves = docs.join("shelves");

        Ok(match name {
            Some(name) => shelves.join(name),
            None => shelves,
        })
    }
    /// Validates a proposed shelf name for filesystem safety.
    ///
    /// - Trims whitespace and ensures it is not empty
    /// - Rejects forbidden characters (`/`, `\`, `:`, `"`, `*`, `?`, `<`, `>`, `|`)
    ///
    /// Returns the sanitized name on success,
    /// or [`ShelfError::InvalidInput`] if validation fails.
    fn valid_shelf(shelf: &str) -> Result<String, ShelfError> {
        let trimmed = shelf.trim();

        if trimmed.is_empty() {
            return Err(ShelfError::InvalidInput);
        }

        if trimmed.contains(&['/', '\\', ':', '"', '*', '?', '<', '>', '|'][..]) {
            return Err(ShelfError::InvalidInput);
        }

        Ok(trimmed.to_owned())
    }
}
