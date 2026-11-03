#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Store(#[from] lh_store::Error),
    #[error("no such {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("file problem: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
