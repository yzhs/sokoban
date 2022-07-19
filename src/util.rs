use std::io;
use std::path::PathBuf;

use directories::{ProjectDirs};

pub const TITLE: &str = "Sokoban";

lazy_static! {
    pub static ref DATA_DIR: PathBuf = {
        let proj_dirs = ProjectDirs::from("de", "yzhs", "sokoban").unwrap();
        proj_dirs.data_dir().into()
    };

    /// Path to the assets directory
    pub static ref ASSETS: PathBuf = ::find_folder::Search::ParentsThenKids(3, 3)
            .for_folder("assets")
            .unwrap();

}

#[derive(Debug, thiserror::Error)]
pub enum SokobanError {
    #[error("I/O error: {0}")]
    IoError(String),

    #[error("XML error: {0}")]
    XmlError(String),

    #[error("No worker in level #{0}")]
    NoWorker(usize),

    #[error("More than one worker in level #{0}")]
    TwoWorkers(usize),

    #[error("Level #{0}: #crates - #goals = {1}")]
    CratesGoalsMismatch(usize, i32),

    #[error("Empty description for level #{0}")]
    NoLevel(usize),
}

/// Automatically wrap io errors
impl From<io::Error> for SokobanError {
    fn from(err: io::Error) -> SokobanError {
        SokobanError::IoError(err.to_string())
    }
}

/// Automatically wrap XML reader errors
impl From<quick_xml::Error> for SokobanError {
    fn from(e: quick_xml::Error) -> Self {
        SokobanError::XmlError(e.to_string())
    }
}
