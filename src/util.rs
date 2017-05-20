use std::error::Error;
use std::fmt;
use std::fs::{File, create_dir_all};
use std::io;
use std::path::Path;

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

/// Open a file, creating it if necessary, in a given directory. If the directory does not exist,
/// create it first.
pub fn create_file_in_dir<P: AsRef<Path>>(dir: P, name: &str, extension: &str) -> File {
    // TODO error handling
    let mut path = dir.as_ref().to_path_buf();
    path.push(name);
    path.set_extension(extension);

    create_dir_all(dir).unwrap();
    File::create(path).unwrap()
}
