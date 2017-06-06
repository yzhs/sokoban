use std::convert::TryFrom;
use std::io::{self, Read};
use std::fmt;
use std::error;
use std::fs::File;
use std::path::PathBuf;

use command::*;
use direction::*;
use level::*;
use save::*;
use util::*;

/// A collection of levels.
#[derive(Debug)]
pub struct Collection {
    pub name: String,
    pub current_level: Level,
    levels: Vec<Level>,
    pub saved: CollectionState,
}

impl Collection {
    /// Load a file containing a bunch of levels separated by an empty line.
    pub fn load(name: &str) -> Result<Collection, SokobanError> {
        let mut level_path = ASSETS.clone();
        level_path.push("levels");
        level_path.push(name);
        level_path.set_extension("lvl");

        let mut level_file = File::open(level_path)?;
        let mut content = "".to_string();
        level_file.read_to_string(&mut content)?;
        let level_strings: Vec<_> = content
            .split("\n\n")
            .map(|x| x.trim_matches('\n'))
            .filter(|x| !x.is_empty())
            .collect();
        let name = level_strings[0];

        let levels = level_strings[1..]
            .iter()
            .enumerate()
            .map(|(i, l)| Level::parse(i, l))
            .collect::<Result<Vec<_>, _>>()?;

        let state = CollectionState::load(name);
        let levels_finished = state.levels_finished();
        let current_level = if state.collection_solved {
            info!("The collection has already been solved.");
            levels[0].clone()
        } else {
            levels[levels_finished].clone()
        };

        let result = Collection {
            name: name.to_string(),
            current_level,
            levels,
            saved: state,
        };
        Ok(result)
    }

    pub fn number_of_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn levels(&self) -> &[Level] {
        &self.levels
    }

    fn reset_level(&mut self) -> Response {
        let n = self.current_level.rank;
        self.current_level = self.levels[n - 1].clone();
        Response::NewLevel(n)
    }

    /// If `current_level` is finished, switch to the next level.
    fn next_level(&mut self) -> Result<Vec<Response>, NextLevelError> {
        let n = self.current_level.rank;
        let finished = self.current_level.is_finished();
        if finished {
            if n == self.levels.len() {
                self.saved.collection_solved = true;
            }

            // Save information on old level
            if let Err(e) = self.save() {
                error!("Failed to create data file: {}", e);
            }

            if n < self.levels.len() {
                self.current_level = self.levels[n].clone();
                Ok(vec![Response::NewLevel(n + 1)])
            } else {
                Err(NextLevelError::EndOfCollection)
            }
        } else if self.saved.levels.len() >= n && n < self.levels.len() {
            self.current_level = self.levels[n].clone();
            Ok(vec![Response::NewLevel(n + 1)])
        } else {
            Err(NextLevelError::LevelNotFinished)
        }
    }

    fn previous_level(&mut self) -> Result<Vec<Response>, ()> {
        let n = self.current_level.rank;
        if n < 2 {
            Err(())
        } else {
            self.current_level = self.levels[n - 2].clone();
            Ok(vec![Response::NewLevel(n - 1)])
        }
    }

    /// Execute whatever command we get from the frontend.
    pub fn execute(&mut self, command: Command) -> Vec<Response> {
        use Command::*;
        let mut result = match command {
            Command::Nothing => vec![],
            Move(dir) => self.current_level.try_move(dir).unwrap_or_default(),
            MoveAsFarAsPossible(dir, MayPushCrate(b)) => {
                self.current_level
                    .move_until(dir, b)
                    .unwrap_or_default()
            }
            MoveToPosition(pos, MayPushCrate(b)) => {
                self.current_level.move_to(pos, b).unwrap_or_default()
            }
            Undo => self.current_level.undo().unwrap_or_default(),
            Redo => self.current_level.redo().unwrap_or_default(),
            ResetLevel => vec![self.reset_level()],
            NextLevel => self.next_level().unwrap_or_default(),
            PreviousLevel => self.previous_level().unwrap_or_default(),
            LoadCollection(_) => unreachable!(),
            Save => {
                self.save().unwrap();
                vec![]
            }
        };
        if self.current_level.is_finished() {
            result.push(Response::LevelFinished);
        }
        result
    }

    /// Find out which direction the worker is currently facing.
    pub fn worker_direction(&self) -> Direction {
        self.current_level.worker_direction()
    }

    // TODO self should not be mut
    pub fn save(&mut self) -> Result<(), SaveError> {
        let rank = self.current_level.rank;
        let level_state = match Solution::try_from(&self.current_level) {
            Ok(soln) => LevelState::new(soln),
            _ => LevelState::Started(self.current_level.clone()),
        };
        self.saved.update(rank - 1, level_state);

        let mut path = PathBuf::new();
        path.push("sokoban");
        path.push(&self.name);
        path.set_extension("json");
        match BASE_DIR
                  .place_data_file(path.as_path())
                  .and_then(File::create) {
            Err(e) => Err(SaveError::from(e)),
            Ok(file) => ::serde_json::to_writer(file, &self.saved).map_err(SaveError::from),
        }
    }
}

#[derive(Debug)]
pub enum NextLevelError {
    LevelNotFinished,
    EndOfCollection,
}

#[derive(Debug)]
pub enum SaveError {
    FailedToCreateFile(io::Error),
    FailedToWriteFile(::serde_json::Error),
}

impl error::Error for SaveError {
    fn description(&self) -> &str {
        use SaveError::*;
        match *self {
            FailedToCreateFile(_) => "Failed to create file",
            FailedToWriteFile(_) => "Failed to serialize to file",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        use SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => e.cause(),
            FailedToWriteFile(ref e) => e.cause(),
        }
    }
}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        SaveError::FailedToCreateFile(e)
    }
}
impl From<::serde_json::Error> for SaveError {
    fn from(e: ::serde_json::Error) -> Self {
        SaveError::FailedToWriteFile(e)
    }
}

impl fmt::Display for SaveError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => write!(fmt, "Failed to create file: {}", e),
            FailedToWriteFile(ref e) => write!(fmt, "Failed to write file: {}", e),
        }
    }
}
