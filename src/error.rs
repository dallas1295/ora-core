use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoughError {
    #[error(transparent)]
    Note(#[from] crate::domain::NoteError),

    #[error(transparent)]
    Shelf(#[from] crate::shelf::storage::ShelfError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
    // TODO implement both sync and db modules
    // #[error(transparent)]
    // Sync(#[from] crate::sync::SyncError),
    // #[error(transparent)]
    // Database(#[from] crate::db::DbError),
}

pub type RoughResult<T> = Result<T, RoughError>;
