use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoughError {
    #[error(transparent)]
    Note(#[from] crate::domain::NoteError),

    #[error(transparent)]
    Shelf(#[from] crate::shelf::storage::ShelfError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("Database connection failed: {0}")]
    Connection(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type RoughResult<T> = Result<T, RoughError>;
