//! On-disc structures for storing which levels have been solved and the best solutions so far.

use std::cmp::Ordering;
use std::convert::TryFrom;
use std::error;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

use rmp_serde::Deserializer;
use serde::Deserialize;

use level::*;
use util::DATA_DIR;

#[derive(Debug, Clone, Copy)]
pub enum UpdateResponse {
    FirstTimeSolved,
    Update { moves: bool, pushes: bool },
}

/// One particular solution of a level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    number_of_moves: usize,
    number_of_pushes: usize,
    steps: String,
}

impl Solution {
    /// Return a copy of either `self` or `other` with the smallest number of *worker* movements.
    pub fn min_moves(&self, other: &Solution) -> Self {
        match self.number_of_moves.cmp(&other.number_of_moves) {
            Ordering::Less => self.clone(),
            Ordering::Equal if self.number_of_pushes <= other.number_of_pushes => self.clone(),
            _ => other.clone(),
        }
    }

    /// Return a copy of either `self` or `other` with the smallest number of *crate* movements.
    pub fn min_pushes(&self, other: &Solution) -> Self {
        match self.number_of_pushes.cmp(&other.number_of_pushes) {
            Ordering::Less => self.clone(),
            Ordering::Equal if self.number_of_moves <= other.number_of_moves => self.clone(),
            _ => other.clone(),
        }
    }

    /// Is `self` a solution with less moves than `other`?
    pub fn less_moves(&self, other: &Solution) -> bool {
        self.number_of_moves < other.number_of_moves
    }

    /// Is `self` a solution with less pushes than `other`?
    pub fn less_pushes(&self, other: &Solution) -> bool {
        self.number_of_pushes < other.number_of_pushes
    }
}

impl<'a> TryFrom<&'a Level> for Solution {
    type Error = ();
    fn try_from(lvl: &'a Level) -> Result<Solution, ()> {
        if lvl.is_finished() {
            Ok(Solution {
                   number_of_moves: lvl.number_of_moves(),
                   number_of_pushes: lvl.number_of_pushes(),
                   steps: lvl.moves_to_string(),
               })
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LevelState {
    /// The level has not been finished.
    Started {
        #[serde(default)]
        rank: usize,
        number_of_moves: usize,
        moves: String,
    },

    /// The level has been finished.
    Finished {
        #[serde(default)]
        rank: usize,
        /// The solution using the least number of moves.
        least_moves: Solution,

        /// The solution using the least number of pushes.
        least_pushes: Solution,
    },
}

impl LevelState {
    pub fn new_solved(rank: usize, solution: Solution) -> Self {
        LevelState::Finished {
            rank,
            least_moves: solution.clone(),
            least_pushes: solution,
        }
    }

    pub fn new_unsolved(level: &Level) -> Self {
        LevelState::Started {
            rank: level.rank,
            number_of_moves: level.number_of_moves(),
            moves: level.all_moves_to_string(),
        }
    }

    /// Does this contain a complete solution?
    pub fn is_finished(&self) -> bool {
        if let LevelState::Started { .. } = *self {
            false
        } else {
            true
        }
    }

    pub fn rank(&self) -> usize {
        match *self {
            LevelState::Started { rank, .. } |
            LevelState::Finished { rank, .. } => rank,
        }
    }

    pub fn set_rank(&mut self, new_rank: usize) {
        match *self {
            LevelState::Started { ref mut rank, .. } |
            LevelState::Finished { ref mut rank, .. } => *rank = new_rank,
        }
    }
}

impl<'a> From<&'a Level> for LevelState {
    fn from(lvl: &'a Level) -> Self {
        if lvl.is_finished() {
            let soln = Solution::try_from(lvl).unwrap();
            LevelState::new_solved(lvl.rank, soln)
        } else {
            LevelState::new_unsolved(lvl)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionState {
    pub name: String,
    pub collection_solved: bool,
    pub levels: Vec<LevelState>,
}

impl CollectionState {
    /// Create a new `CollectionState` with no solved levels.
    pub fn new(name: &str) -> Self {
        CollectionState {
            name: name.to_string(),
            collection_solved: false,
            levels: vec![],
        }
    }

    /// Try to load the `CollectionState` for the level set with the given name. If that fails,
    /// return a new empty `CollectionState`.
    pub fn load(name: &str) -> Self {
        let path = DATA_DIR.join(name);

        Self::load_messagepack(&path)
            .or_else(|| Self::load_json(&path))
            .unwrap_or_else(|| Self::new(name))
    }

    fn load_json(path: &Path) -> Option<Self> {
        info!("Trying to load JSON");
        File::open(path.with_extension("json"))
            .ok()
            .and_then(|file| ::serde_json::from_reader(file).ok())
    }

    fn load_messagepack(path: &Path) -> Option<Self> {
        use std::io::BufReader;
        info!("Trying to load MessagePack");
        File::open(path.with_extension("mp"))
            .ok()
            .and_then(|file| {
                          let mut de = Deserializer::new(BufReader::new(file));
                          Deserialize::deserialize(&mut de).ok()
                      })
    }

    /// Save the current state to disc.
    pub fn save(&mut self, name: &str) -> Result<(), SaveError> {
        // If no rank was given in the JSON file, set it.
        if self.levels[0].rank() == 0 {
            for (i, lvl) in self.levels.iter_mut().enumerate() {
                lvl.set_rank(i + 1);
            }
        }

        let mut path = DATA_DIR.join(name);
        path.set_extension("json");
        File::create(path)
            .map_err(SaveError::from)
            .and_then(|file| ::serde_json::to_writer(file, &self).map_err(SaveError::from))
            .map(|_| ())
    }

    /// If a better or more complete solution for the current level is available, replace the old
    /// one with it.
    pub fn update(&mut self, index: usize, level_state: LevelState) -> UpdateResponse {
        if index >= self.levels.len() {
            self.levels.push(level_state);
            UpdateResponse::FirstTimeSolved
        } else {
            use self::LevelState::*;
            let ls_old = self.levels[index].clone();
            match ls_old {
                Started { .. } => {
                    self.levels[index] = level_state;
                    UpdateResponse::FirstTimeSolved
                }
                Finished {
                    rank,
                    least_moves: ref lm_old,
                    least_pushes: ref lp_old,
                } => {
                    if let Finished {
                               least_moves: ref lm,
                               least_pushes: ref lp,
                               ..
                           } = level_state {
                        self.levels[index] = Finished {
                            rank,
                            least_moves: lm_old.min_moves(lm),
                            least_pushes: lp_old.min_pushes(lp),
                        };
                        let highscore_moves = lm_old.less_moves(lm);
                        let highscore_pushes = lp_old.less_pushes(lp);
                        UpdateResponse::Update {
                            moves: highscore_moves,
                            pushes: highscore_pushes,
                        }
                    } else {
                        UpdateResponse::Update {
                            moves: false,
                            pushes: false,
                        }
                    }
                }
            }
        }
    }

    /// How many levels have been finished.
    pub fn levels_finished(&self) -> usize {
        let n = self.levels.len();
        if n == 0 || !self.levels[0].is_finished() {
            0
        } else if self.levels[n - 1].is_finished() {
            n
        } else {
            n - 1
        }
    }
}

#[derive(Debug)]
pub enum SaveError {
    FailedToCreateFile(io::Error),
    FailedToWriteFile(::serde_json::Error),
}

impl error::Error for SaveError {
    fn description(&self) -> &str {
        use self::SaveError::*;
        match *self {
            FailedToCreateFile(_) => "Failed to create file",
            FailedToWriteFile(_) => "Failed to serialize to file",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        use self::SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => e.cause(),
            FailedToWriteFile(ref e) => e.cause(),
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

impl fmt::Display for SaveError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use self::SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => write!(fmt, "Failed to create file: {}", e),
            FailedToWriteFile(ref e) => write!(fmt, "Failed to write file: {}", e),
        }
    }
}
