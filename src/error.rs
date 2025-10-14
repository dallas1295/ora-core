use thiserror::Error;

pub type OraResult<T> = Result<T, OraError>;

#[derive(Debug, Error)]
pub enum OraError {
    #[error("no changes to file")]
    NoChanges,

    #[error(transparent)]
    Note(crate::domain::NoteError),

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
