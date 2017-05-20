use std::convert::TryFrom;
use std::io::Read;
use std::fs::File;
use std::path::Path;

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
    pub fn load<P: AsRef<Path>>(assets_path: P, name: &str) -> Result<Collection, SokobanError> {
        let mut level_path = assets_path.as_ref().to_path_buf();
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
        info!("Loading level {}", levels_finished + 1);
        let result = Collection {
            name: name.to_string(),
            current_level: levels[levels_finished].clone(),
            levels,
            saved: state,
        };
        Ok(result)
    }

    pub fn number_of_levels(&self) -> usize {
        self.levels.len()
    }

    /// If `current_level` is finished, switch to the next level.
    pub fn next_level(&mut self) -> Result<Vec<Response>, NextLevelError> {
        let n = self.current_level.rank;
        let finished = self.current_level.is_finished();
        if finished {
            // Save information on old level
            self.save();

            if n < self.levels.len() {
                self.current_level = self.levels[n].clone();
                Ok(vec![Response::NewLevel(n + 1)])
            } else {
                self.saved.collection_solved = true;
                Err(NextLevelError::EndOfCollection)
            }
        } else {
            Err(NextLevelError::LevelNotFinished)
        }
    }

    /// Execute whatever command we get from the frontend.
    pub fn execute(&mut self, command: Command) -> Vec<Response> {
        use Command::*;
        let mut result = match command {
            Command::Nothing => vec![],
            Move(dir) => {
                self.current_level
                    .try_move(dir)
                    .unwrap_or_else(|_| vec![])
            }
            MoveAsFarAsPossible(dir, MayPushCrate(b)) => {
                self.current_level
                    .move_until(dir, b)
                    .unwrap_or_else(|_| vec![])
            }
            MoveToPosition(pos, MayPushCrate(b)) => {
                self.current_level
                    .move_to(pos, b)
                    .unwrap_or_else(|_| vec![])
            }
            Undo => self.current_level.undo().unwrap_or_else(|_| vec![]),
            Redo => self.current_level.redo().unwrap_or_else(|_| vec![]),
            NextLevel => self.next_level().unwrap_or_else(|_| vec![]),
            PreviousLevel => unimplemented!(),
            LoadCollection(name) => {
                error!("Loading level collection {} is not implemented!", name);
                unimplemented!()
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

    pub fn save(&mut self) {
        let rank = self.current_level.rank;
        let level_state = match Solution::try_from(&self.current_level) {
            Ok(soln) => LevelState::new(soln),
            _ => LevelState::Started(self.current_level.clone()),
        };
        self.saved.update(rank - 1, level_state);

        let file = create_file_in_dir("/home/yzhs/.local/share/sokoban", &self.name, "json");
        if let Err(e) = ::serde_json::to_writer(file, &self.saved) {
            error!("Failed to write savegame: {}", e);
        }
    }
}

pub enum NextLevelError {
    LevelNotFinished,
    EndOfCollection,
}
