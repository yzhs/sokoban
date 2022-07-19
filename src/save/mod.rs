//! On-disc structures for storing which levels have been solved and the best solutions so far.

mod collection_state;
mod level_state;
mod solution;

use std::io;

pub use self::collection_state::*;
pub use self::level_state::*;
pub use self::solution::*;

#[derive(Debug, Clone, Copy)]
pub enum UpdateResponse {
    FirstTimeSolved,
    Update { moves: bool, pushes: bool },
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("Failed to create file: {0}")]
    FailedToCreateFile(String),

    #[error("Failed to create CBOR: {0}")]
    CBOREncodeError(String),
}


impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        self::SaveError::FailedToCreateFile(e.to_string())
    }
}

impl From<::serde_cbor::error::Error> for SaveError {
    fn from(e: ::serde_cbor::error::Error) -> Self {
        self::SaveError::CBOREncodeError(e.to_string())
    }
}
