use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Internal(String),
}
