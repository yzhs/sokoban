use std::io;
use std::error::Error;
use std::fmt;

pub const ASSETS_PATH: &'static str = "/home/yzhs/prj/sokoban/assets";

#[derive(Debug)]
pub enum SokobanError {
    IoError(io::Error),
    NoWorker,
    TwoWorkers,
    CratesGoalsMismatch(usize, i32),
}

impl fmt::Display for SokobanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::SokobanError::*;
        match self {
            &IoError(ref err) => write!(f, "{}", err),
            &NoWorker => write!(f, "NoWorker"),
            &TwoWorkers => write!(f, "TwoWorkers"),
            &CratesGoalsMismatch(lvl, goals_minus_crates) => {
                write!(f, "CratesGoalsMismatch({}, {})", lvl, goals_minus_crates)
            }
        }
    }
}

impl Error for SokobanError {
    #[doc(hidden)]
    fn description(&self) -> &str {
        use self::SokobanError::*;
        match self {
            &IoError(ref err) => err.description(),
            &TwoWorkers => "More than one worker found.",
            &NoWorker => "No worker found.",
            &CratesGoalsMismatch(_, _) => "The number of crates and goals does not match",
        }
    }
}

// Automatically wrap io errors
impl From<io::Error> for SokobanError {
    fn from(err: io::Error) -> SokobanError {
        SokobanError::IoError(err)
    }
}
