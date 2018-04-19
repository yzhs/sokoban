//! On-disc structures for storing which levels have been solved and the best solutions so far.

mod collection_state;
mod level_state;
mod solution;

use std::error;
use std::fmt;
use std::io;

pub use self::collection_state::*;
pub use self::level_state::*;
pub use self::solution::*;

#[derive(Debug, Clone, Copy)]
pub enum UpdateResponse {
    FirstTimeSolved,
    Update { moves: bool, pushes: bool },
}

#[derive(Debug)]
pub enum SaveError {
    FailedToCreateFile(io::Error),
    FailedToWriteFile(::serde_json::Error),
    CBOREncodeError(::serde_cbor::error::Error),
}

impl error::Error for SaveError {
    fn description(&self) -> &str {
        use self::SaveError::*;
        match *self {
            FailedToCreateFile(_) => "Failed to create file",
            FailedToWriteFile(_) => "Failed to serialize to file",
            CBOREncodeError(_) => "Failed to serialize to CBOR",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        use self::SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => e.cause(),
            FailedToWriteFile(ref e) => e.cause(),
            CBOREncodeError(ref e) => e.cause(),
        }
    }
}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        self::SaveError::FailedToCreateFile(e)
    }
}
impl From<::serde_json::Error> for SaveError {
    fn from(e: ::serde_json::Error) -> Self {
        self::SaveError::FailedToWriteFile(e)
    }
}
impl From<::serde_cbor::error::Error> for SaveError {
    fn from(e: ::serde_cbor::error::Error) -> Self {
        self::SaveError::CBOREncodeError(e)
    }
}

impl fmt::Display for SaveError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => write!(fmt, "Failed to create file: {}", e),
            FailedToWriteFile(ref e) => write!(fmt, "Failed to write file: {}", e),
            CBOREncodeError(ref e) => write!(fmt, "Failed to encode CBOR file: {}", e),
        }
    }
}
