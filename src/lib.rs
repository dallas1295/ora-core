//! # ora_core
//!
//! A powerful Rust library for managing local Markdown notes with real-time file system watching,
//! full-text search, and SQLite-based indexing.
//!
//! ## Features
//!
//! - **Note Management**: Create, read, update, and delete Markdown notes with atomic file operations
//! - **Shelf Organization**: Organize notes in shelf-based directories with validation
//! - **Real-time Watching**: Monitor file system changes with debounced event processing
//! - **Full-text Search**: SQLite FTS5-powered search with BM25 ranking and snippets
//! - **Robust Error Handling**: Comprehensive error types with automatic conversions
//! - **Thread-safe Operations**: Safe concurrent access to indexed data
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ora_core::{shelf::storage::Shelf, watcher::service::WatcherService};
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new shelf for organizing notes
//! let shelf = Shelf::ensure_exists("my-notes")?;
//! println!("Created shelf at: {}", shelf.root.display());
//!
//! // Start real-time file watching
//! let mut watcher = WatcherService::create(&shelf.root, Duration::from_millis(100))?;
//! watcher.run()?;
//!
//! // Your application can now create notes and they'll be automatically indexed
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The library is organized into several key modules:
//!
//! - **[`domain`]**: Core note operations and file management
//! - **[`shelf`]**: Shelf storage and management functionality
//! - **[`watcher`]: Real-time file system monitoring and indexing
//! - **[`search`]: Full-text search with SQLite FTS5
//! - **[`error`]: Unified error handling throughout the library
//!
//! ## Note Management
//!
//! Notes are stored as Markdown files (`.md`) with automatic title extraction
//! from filenames. The library handles:
//!
//! - Atomic file writes to prevent corruption
//! - Automatic filename generation with conflict resolution
//! - Content and title updates with proper file renaming
//! - Safe deletion operations
//!
//! ```rust,no_run
//! use ora_core::domain::LocalNote;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let notes_dir = Path::new("/path/to/notes");
//!
//! // Create a new note
//! let note = LocalNote::create(
//!     "My First Note",
//!     "# My First Note\n\nThis is the content.",
//!     notes_dir
//! )?;
//!
//! // Update the note
//! let mut updated_note = note.with_content("Updated content");
//! updated_note.save()?;
//!
//! // Delete the note
//! updated_note.delete()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Search Functionality
//!
//! The search system uses SQLite's FTS5 extension for powerful full-text search:
//!
//! ```rust,no_run
//! use ora_core::search::{Query, SearchOptions};
//! use ora_core::watcher::index::Index;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let index = Index::new(Path::new("/path/to/shelf"))?;
//! let query = Query::new(&index);
//!
//! // Simple search
//! let results = query.search("rust programming")?;
//!
//! // Advanced search with options
//! let options = SearchOptions {
//!     limit: Some(10),
//!     include_snippets: true,
//!     snippet_length: 150,
//!     ..Default::default()
//! };
//! let results = query.search_with_options("title:rust AND tutorial", &options)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## File System Watching
//!
//! The watcher module provides real-time monitoring of file changes:
//!
//! - Debounced event processing to handle rapid changes
//! - Automatic indexing of new and modified files
//! - Proper cleanup of deleted files from the index
//! - Thread-safe operation with graceful shutdown
//!
//! ## Error Handling
//!
//! All operations return [`OraResult<T>`] which wraps the unified [`OraError`] type.
//! The error system automatically converts from sub-module error types,
//! allowing the use of the `?` operator throughout the codebase.
//!
//! ```rust,no_run
//! use ora_core::{OraResult, OraError};
//! use ora_core::domain::LocalNote;
//!
//! fn create_note_safely() -> OraResult<LocalNote> {
//!     // This will automatically convert any NoteError to OraError
//!     let note = LocalNote::create("Title", "Content", std::path::Path::new("/notes"))?;
//!     Ok(note)
//! }
//! ```

pub mod domain;
pub mod error;
pub mod search;
pub mod shelf;
pub mod watcher;

/// Re-exports the most commonly used types for convenience.
pub use error::{OraError, OraResult};
