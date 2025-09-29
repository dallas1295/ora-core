use dirs;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use thiserror::Error;


// TODO Swap out the standard Error output to the custom ShelfErrors.
// TODO ensure that error contexts match their usecase.
#[derive(Debug, Error)]
pub enum ShelfError {
    #[error("directory not found")]
    NotFound,
    #[error("shelf already exists")]
    AlreadyExists,
    #[error("empty or invalid chars")]
    InvalidInput,
    #[error("access permissions denied")]
    PermissionDenied,
    #[error("error read/write")]
    IOError,
}

pub struct Shelf {
    pub root: PathBuf,
    pub name: String,
}

impl Shelf {
    /// Creates a brand-new shelf directory under `~/Documents/shelves/{name}`.
    ///
    /// This validates the shelf name, then attempts to create the directory.
    /// Returns `Err(AlreadyExists)` if the shelf already exists,
    /// otherwise returns a new [`Shelf`] instance with `root` and `name`.
    pub fn new(name: &str) -> Result<Self> {
        let root = Shelf::shelf_path(Some(name))?;
        let shelf_name = Self::valid_shelf(name)?;

        if root.exists() {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("shelf '{}' already exists", root.display()),
            ));
        }

        fs::create_dir_all(&root)?;

        Ok(Self {
            root,
            name: shelf_name,
        })
    }

    /// Opens an existing shelf directory under `~/Documents/shelves/{name}`.
    ///
    /// Returns `Err(NotFound)` if the shelf does not exist, or
    /// `Err(InvalidInput)` if the path exists but is not a directory.
    /// On success, returns a [`Shelf`] pointing to the resolved root.
    pub fn open(name: &str) -> Result<Self> {
        let root = Shelf::shelf_path((name))?;

        if !root.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("shelf {} does not exist", root.display()),
            ));
        }

        if !root.is_dir() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("shelf path is not a directory: {}", root.display()),
            ));
        }

        Ok(Self {
            root,
            name: name.to_string(),
        })
    }

    pub fn list_shelves() -> Result<Vec<String>> {
        let shelf_base = Shelf::shelf_path(None)?;

        let names = fs::read_dir(&shelf_base)?
            .filter_map(|res| {
                let entry = res.map_err(Error::new(
                    ErrorKind::
                ))?;
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
    /// Ensures a shelf exists, creating it if missing.
    ///
    /// If the shelf directory does not exist yet, it will be created.
    /// If it already exists, it will be opened.
    /// Useful for startup or daemon code where creation vs. opening does not matter.
    pub fn ensure_exists(name: &str) -> Result<Self> {
        let root = Shelf::shelf_path(Some(name))?;
        if !root.exists() {
            fs::create_dir_all(&root)?;
        }

        Shelf::open(name)
    }

    pub fn rename(&mut self, new_name: &str) -> Result<()> {
        let valid_new_name = Shelf::valid_shelf(new_name)?;

        let new_path = Self::shelf_path(Some(&valid_new_name))?;

        if new_path.exists() {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("a(n) {new_name} shelf already exists."),
            ));
        }
        fs::rename(&self.root, &new_path)?;

        self.name = valid_new_name;
        self.root = new_path;

        Ok(())
    }

    pub fn delete_shelf(&self) -> Result<()> {
        fs::remove_dir_all(&self.root)
    }

    /// Resolves a given shelf name into a full path under `~/Documents/shelves/{name}`.
    ///
    /// Returns `Err(NotFound)` if the user's home directory cannot be determined.
    fn shelf_path(name: Option<&str>) -> Result<PathBuf> {
        let docs = dirs::document_dir()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "documents directory not found"))?;

        let shelves = docs.join("shelves");

        let path = match name {
            Some(name) => shelves.join(name),
            None => shelves,
        };

        Ok(path)
    }

    /// Validates a proposed shelf name for filesystem safety.
    ///
    /// - Trims whitespace and ensures it is not empty.
    /// - Rejects characters not valid in filenames (`/`, `\`, `:`, `"`, `*`, `?`, `<`, `>`, `|`).
    ///
    /// Returns `Err(InvalidInput)` on invalid names; otherwise returns the cleaned name.
    fn valid_shelf(shelf: &str) -> Result<String> {
        let trimmed = shelf.trim();

        if trimmed.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("shelf name cannot be empty."),
            ));
        }

        if trimmed.contains(&['/', '\\', ':', '"', '*', '?', '<', '>', '|'][..]) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("invalid char in shelf name."),
            ));
        }

        Ok(trimmed.to_owned())
    }
}
