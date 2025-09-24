use dirs;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;

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
        let root = Shelf::shelf_path(name)?;
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
        let root = Shelf::shelf_path(name)?;

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

    /// Ensures a shelf exists, creating it if missing.
    ///
    /// If the shelf directory does not exist yet, it will be created.
    /// If it already exists, it will be opened.
    /// Useful for startup or daemon code where creation vs. opening does not matter.
    pub fn ensure_exists(name: &str) -> Result<Self> {
        let root = Shelf::shelf_path(name)?;
        if !root.exists() {
            fs::create_dir_all(&root)?;
        }

        Shelf::open(name)
    }

    /// Resolves a given shelf name into a full path under `~/Documents/shelves/{name}`.
    ///
    /// Returns `Err(NotFound)` if the user's home directory cannot be determined.
    fn shelf_path(name: &str) -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "home directory not found"))?;
        Ok(home.join("Documents/shelves").join(name))
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
