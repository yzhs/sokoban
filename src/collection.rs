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
    /// The full name of the collection.
    pub name: String,

    /// The name of the file containing the level collection.
    pub short_name: String,

    /// A copy of one of the levels.
    pub current_level: Level,

    /// All levels of this collection. This variable is only written to when loading the
    /// collection.
    levels: Vec<Level>,

    /// What levels have been solved and with how many moves/pushes.
    saved: CollectionState,
}

impl Collection {
    /// Load a file containing a bunch of levels separated by an empty line.
    pub fn load(short_name: &str) -> Result<Collection, SokobanError> {
        let mut level_path = ASSETS.clone();
        level_path.push("levels");
        level_path.push(short_name);
        level_path.set_extension("lvl");

        // Read the collection’s file
        let mut level_file = File::open(level_path)?;
        let mut content = "".to_string();
        level_file.read_to_string(&mut content)?;
        let level_strings: Vec<_> = content
            .split("\n\n")
            .map(|x| x.trim_matches('\n'))
            .filter(|x| !x.is_empty())
            .collect();
        let name = level_strings[0];

        // Parse the individual levels
        let levels = level_strings[1..]
            .iter()
            .enumerate()
            .map(|(i, l)| Level::parse(i, l))
            .collect::<Result<Vec<_>, _>>()?;

        // Try to load the collection’s status
        let state = CollectionState::load(short_name);
        let current_level = if state.collection_solved {
            info!("The collection has already been solved.");
            levels[0].clone()
        } else {
            levels[state.levels_finished()].clone()
        };

        let result = Collection {
            name: name.to_string(),
            short_name: short_name.to_string(),
            current_level,
            levels,
            saved: state,
        };
        Ok(result)
    }

    // Accessor methods

    /// Is the current level the last one in this collection?
    pub fn end_of_collection(&self) -> bool {
        self.current_level.rank == self.levels.len()
    }

    /// Find out which direction the worker is currently facing.
    pub fn worker_direction(&self) -> Direction {
        self.current_level.worker_direction()
    }
}


impl Collection {
    /// Execute whatever command we get from the frontend.
    pub fn execute(&mut self, command: Command) -> Vec<Response> {
        use Command::*;
        let mut result = match command {
            Command::Nothing => vec![],
            Move(dir) => self.current_level.try_move(dir),
            MoveAsFarAsPossible(dir, MayPushCrate(b)) => {
                self.current_level
                    .move_until(dir, b)
                    .unwrap_or_default()
            }
            MoveToPosition(pos, MayPushCrate(b)) => self.current_level.move_to(pos, b),
            Undo => self.current_level.undo(),
            Redo => self.current_level.redo(),
            ResetLevel => vec![self.reset_level()],
            NextLevel => self.next_level().unwrap_or_default(),
            PreviousLevel => self.previous_level().unwrap_or_default(),
            LoadCollection(_) => unreachable!(),
            Save => {
                let _ = self.save().unwrap();
                vec![]
            }
        };
        if self.current_level.is_finished() {
            if self.current_level.rank == self.levels.len() {
                self.saved.collection_solved = true;
            }

            // Save information on old level
            match self.save() {
                Ok(resp) => result.push(Response::LevelFinished(resp)),
                Err(e) => {
                    error!("Failed to create data file: {}", e);
                    result.push(Response::LevelFinished(UpdateResponse::FirstTimeSolved))
                }
            }

        }
        result
    }

    // Helpers for Collection::execute

    /// Replace the current level by a clean copy.
    fn reset_level(&mut self) -> Response {
        let n = self.current_level.rank;
        self.current_level = self.levels[n - 1].clone();
        Response::ResetLevel
    }

    /// If `current_level` is finished, switch to the next level.
    fn next_level(&mut self) -> Result<Vec<Response>, NextLevelError> {
        let n = self.current_level.rank;
        let finished = self.current_level.is_finished();
        if finished {
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

    /// Go to the previous level unless this is already the first level in this collection.
    fn previous_level(&mut self) -> Result<Vec<Response>, ()> {
        let n = self.current_level.rank;
        if n < 2 {
            Err(())
        } else {
            self.current_level = self.levels[n - 2].clone();
            Ok(vec![Response::NewLevel(n - 1)])
        }
    }

    /// Save the state of this collection including the state of the current level.
    fn save(&mut self) -> Result<UpdateResponse, SaveError> {
        // TODO self should not be mut
        let rank = self.current_level.rank;
        let level_state = match Solution::try_from(&self.current_level) {
            Ok(soln) => LevelState::new_solved(soln),
            _ => LevelState::new_unsolved(&self.current_level),
        };
        let response = self.saved.update(rank - 1, level_state);

        let mut path = PathBuf::new();
        path.push(&self.short_name);
        path.set_extension("json");
        match File::create(DATA_DIR.join(path.as_path())) {
            Err(e) => Err(SaveError::from(e)),
            Ok(file) => {
                ::serde_json::to_writer(file, &self.saved)
                    .map_err(SaveError::from)?;
                Ok(response)
            }
        }
    }
}

#[derive(Debug)]
pub enum NextLevelError {
    /// Tried to move to the next levels when the current one has not been solved.
    LevelNotFinished,

    /// Cannot move past the last level of a collection.
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
