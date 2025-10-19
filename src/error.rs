//! Unified error handling for the ora-core library.
//!
//! This module defines the central error type [`OraError`] that encompasses all
//! possible error conditions that can occur throughout the ora-core library.
//! It provides a consistent error interface that wraps domain-specific errors
//! from notes, shelves, database operations, and file system watching.
//!
//! # Error Hierarchy
//!
//! The error system is organized into several categories:
//!
//! - **Note errors**: Issues with note operations (create, read, update, delete)
//! - **Shelf errors**: Problems with shelf management and storage
//! - **Database errors**: SQLite connection and query failures
//! - **I/O errors**: File system operations
//! - **Watcher errors**: File system monitoring issues
//! - **Generic errors**: Catch-all for other error conditions
//!
//! # Conversions
//!
//! The module provides automatic conversions from sub-module error types
//! to [`OraError`] via `From` implementations, allowing the use of the `?`
//! operator throughout the codebase.

use thiserror::Error;

/// A convenient type alias for results that use [`OraError`].
///
/// This is the primary result type used throughout the ora-core library,
/// providing a consistent way to handle operations that can fail.
pub type OraResult<T> = Result<T, OraError>;

/// The unified error type for the ora-core library.
///
/// This enum represents all possible error conditions that can occur when
/// working with notes, shelves, search functionality, and file system watching.
/// It provides a single error type that can be used throughout the application
/// while still preserving specific error information.
#[derive(Debug, Error)]
pub enum OraError {
    /// No changes were made to a file during a save operation.
    ///
    /// This error is returned when attempting to save a note that already
    /// contains the same content as the file on disk.
    #[error("no changes to file")]
    NoChanges,

    /// Errors related to note operations.
    ///
    /// Wraps the domain-specific `NoteError` type, preserving detailed
    /// information about note-related failures.
    #[error(transparent)]
    Note(crate::domain::NoteError),

    /// Errors related to shelf operations.
    ///
    /// Automatically converted from `ShelfError` for convenience.
    #[error(transparent)]
    Shelf(#[from] crate::shelf::storage::ShelfError),

    /// I/O errors from file system operations.
    ///
    /// Automatically converted from `std::io::Error`.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Database-related errors.
    ///
    /// Wraps SQLite errors from the rusqlite library.
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),

    /// Database connection failures.
    ///
    /// Used for connection-specific errors that don't fit into the standard
    /// rusqlite error categories.
    #[error("Database connection failed: {0}")]
    Connection(String),

    /// Generic error for miscellaneous issues.
    ///
    /// Used as a catch-all for error conditions that don't fit into other
    /// categories. Prefer using more specific error variants when possible.
    #[error("Other error: {0}")]
    Other(String),

    /// File system watcher errors.
    ///
    /// Automatically converted from notify library errors.
    #[error(transparent)]
    Watcher(#[from] notify::Error),
}

/// Automatic conversion from `NoteError` to `OraError`.
///
/// This implementation allows the use of the `?` operator when working
/// with note operations, automatically converting domain-specific errors
/// into the unified error type.
impl From<crate::domain::NoteError> for OraError {
    fn from(err: crate::domain::NoteError) -> Self {
        match err {
            crate::domain::NoteError::NoChanges => OraError::NoChanges,
            crate::domain::NoteError::InvalidPath => {
                OraError::Note(crate::domain::NoteError::InvalidPath)
            }
            crate::domain::NoteError::Io(io_error) => {
                OraError::Note(crate::domain::NoteError::Io(io_error))
            }
        }
    }
}
