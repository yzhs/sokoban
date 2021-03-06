use std::fs::File;
use std::path::Path;

use crate::util::DATA_DIR;

use super::level_state::*;
use super::{SaveError, UpdateResponse};

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionState {
    pub name: String,

    pub collection_solved: bool,

    #[serde(default)]
    pub levels_solved: u32,

    pub levels: Vec<LevelState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatsOnlyCollectionState {
    pub name: String,

    pub collection_solved: bool,

    #[serde(default)]
    pub levels_solved: u32,
}

impl CollectionState {
    /// Create a new `CollectionState` with no solved levels.
    pub fn new(name: &str) -> Self {
        CollectionState {
            name: name.to_string(),
            collection_solved: false,
            levels_solved: 0,
            levels: vec![],
        }
    }

    pub fn load_stats(name: &str) -> Self {
        Self::load_helper(name, true)
    }

    /// Try to load the `CollectionState` for the level set with the given name. If that fails,
    /// return a new empty `CollectionState`.
    pub fn load(name: &str) -> Self {
        Self::load_helper(name, false)
    }

    fn load_helper(name: &str, stats_only: bool) -> Self {
        let path = DATA_DIR.join(name);

        Self::load_cbor(&path, stats_only)
            .or_else(|| Self::load_json(&path, stats_only))
            .unwrap_or_else(|| Self::new(name))
    }

    fn from_stats(stats: StatsOnlyCollectionState) -> Self {
        Self {
            name: stats.name,
            collection_solved: stats.collection_solved,
            levels_solved: stats.levels_solved,
            levels: vec![],
        }
    }

    fn load_json(path: &Path, stats_only: bool) -> Option<Self> {
        let file = File::open(path.with_extension("json")).ok();

        if stats_only {
            let stats: Option<StatsOnlyCollectionState> =
                file.and_then(|file| ::serde_json::from_reader(file).ok());
            stats.map(Self::from_stats)
        } else {
            file.and_then(|file| ::serde_json::from_reader(file).ok())
        }
    }

    fn load_cbor(path: &Path, stats_only: bool) -> Option<Self> {
        let file = File::open(path.with_extension("cbor")).ok()?;

        let result = if stats_only {
            let stats: StatsOnlyCollectionState =
                serde_cbor::from_reader(file).expect("Could not read cbor file");
            Self::from_stats(stats)
        } else {
            serde_cbor::from_reader(file).expect("Could not read cbor file")
        };

        Some(result)
    }

    /// Save the current state to disc.
    pub fn save(&mut self, name: &str) -> Result<(), SaveError> {
        self.levels_solved = self.levels_finished() as u32;

        self.save_cbor(name)
    }

    fn save_cbor(&self, name: &str) -> Result<(), SaveError> {
        let mut path = DATA_DIR.join(name);
        path.set_extension("cbor");
        File::create(path)
            .map_err(SaveError::from)
            .and_then(|mut file| ::serde_cbor::to_writer(&mut file, &self).map_err(SaveError::from))
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
            match self.levels[index].clone() {
                Started { .. } => {
                    self.levels[index] = level_state;
                    UpdateResponse::FirstTimeSolved
                }
                Finished {
                    least_moves: ref lm_old,
                    least_pushes: ref lp_old,
                } => {
                    if let Finished {
                        least_moves: ref lm,
                        least_pushes: ref lp,
                        ..
                    } = level_state
                    {
                        self.levels[index] = Finished {
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

    pub fn number_of_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn number_of_solved_levels(&self) -> usize {
        if self.levels.is_empty() {
            self.levels_solved as usize
        } else {
            self.levels_finished()
        }
    }
}
