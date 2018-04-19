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

#[derive(Debug, Fail)]
pub enum SaveError {
    #[fail(display = "Failed to create file: {}", _0)]
    FailedToCreateFile(io::Error),

    #[fail(display = "Failed to create CBOR: {}", _0)]
    CBOREncodeError(::serde_cbor::error::Error),
}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        self::SaveError::FailedToCreateFile(e)
    }
}

impl From<::serde_cbor::error::Error> for SaveError {
    fn from(e: ::serde_cbor::error::Error) -> Self {
        self::SaveError::CBOREncodeError(e)
    }
}
