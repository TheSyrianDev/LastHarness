#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Store(#[from] lh_store::Error),
    #[error(transparent)]
    Learn(#[from] lh_learn::Error),
    #[error("file problem: {0}")]
    Io(#[from] std::io::Error),
    #[error("no such {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    /// A model type is used by roles, so each of them must be given somewhere else to go first.
    #[error("these roles use the type and need another: {0:?}")]
    NeedsReassign(Vec<String>),
    /// The model a role asks for is not available and the setting says to stop.
    #[error("{0}")]
    Unavailable(String),
}
pub type Result<T> = std::result::Result<T, Error>;
