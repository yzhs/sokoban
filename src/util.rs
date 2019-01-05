use std::io;
use std::path::PathBuf;

use app_dirs::{app_dir, AppDataType, AppInfo};
use quick_xml;

pub const TITLE: &str = "Sokoban";

const APP_INFO: AppInfo = AppInfo {
    name: "sokoban",
    author: "yzhs",
};

lazy_static! {
    pub static ref DATA_DIR: PathBuf = app_dir(AppDataType::UserData, &APP_INFO, "").unwrap();

    /// Path to the assets directory
    pub static ref ASSETS: PathBuf = ::find_folder::Search::ParentsThenKids(3, 3)
            .for_folder("assets")
            .unwrap();

}

#[derive(Debug, Fail)]
pub enum SokobanError {
    #[fail(display = "I/O error: {}", _0)]
    IoError(io::Error),

    #[fail(display = "XML error: {}", _0)]
    XmlError(quick_xml::Error),

    #[fail(display = "No worker in level #{}", _0)]
    NoWorker(usize),

    #[fail(display = "More than one worker in level #{}", _0)]
    TwoWorkers(usize),

    #[fail(display = "Level #{}: #crates - #goals = {}", _0, _1)]
    CratesGoalsMismatch(usize, i32),

    #[fail(display = "Empty description for level #{}", _0)]
    NoLevel(usize),
}

/// Automatically wrap io errors
impl From<io::Error> for SokobanError {
    fn from(err: io::Error) -> SokobanError {
        SokobanError::IoError(err)
    }
}

/// Automatically wrap XML reader errors
impl From<quick_xml::Error> for SokobanError {
    fn from(e: quick_xml::Error) -> Self {
        SokobanError::XmlError(e)
    }
}
