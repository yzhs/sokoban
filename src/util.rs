use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

lazy_static!{
    pub static ref BASE_DIR: ::xdg::BaseDirectories = ::xdg::BaseDirectories::new().unwrap();

    /// Path to the assets directory
    pub static ref ASSETS: PathBuf = ::find_folder::Search::ParentsThenKids(3, 3)
            .for_folder("assets")
            .unwrap();

}

#[derive(Debug)]
pub enum SokobanError {
    IoError(io::Error),
    NoWorker(usize),
    TwoWorkers(usize),
    CratesGoalsMismatch(usize, i32),
}

impl fmt::Display for SokobanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::SokobanError::*;
        match *self {
            IoError(ref err) => write!(f, "{}", err),
            NoWorker(lvl) => write!(f, "NoWorker({})", lvl),
            TwoWorkers(lvl) => write!(f, "TwoWorkers({})", lvl),
            CratesGoalsMismatch(lvl, goals_minus_crates) => {
                write!(f, "CratesGoalsMismatch({}, {})", lvl, goals_minus_crates)
            }
        }
    }
}

impl Error for SokobanError {
    #[doc(hidden)]
    fn description(&self) -> &str {
        use self::SokobanError::*;
        match *self {
            IoError(ref err) => err.description(),
            TwoWorkers(_) => "More than one worker found.",
            NoWorker(_) => "No worker found.",
            CratesGoalsMismatch(_, _) => "The number of crates and goals does not match",
        }
    }
}

/// Automatically wrap io errors
impl From<io::Error> for SokobanError {
    fn from(err: io::Error) -> SokobanError {
        SokobanError::IoError(err)
    }
}
