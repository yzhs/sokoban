use std::io::Read;
use std::fs::File;
use std::path::Path;

use command::*;
use direction::*;
use level::*;
use util::*;

/// A collection of levels.
#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub current_level: CurrentLevel,
    pub levels: Vec<Level>,
}

impl Collection {
    /// Load a file containing a bunch of levels separated by an empty line.
    pub fn load(name: &str) -> Result<Collection, SokobanError> {
        let assets_path: &Path = ASSETS_PATH.as_ref();
        let mut level_path = assets_path.to_path_buf();
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
        Ok(Collection {
               name: name.to_string(),
               current_level: CurrentLevel::new(levels[0].clone()),
               levels,
           })
    }

    /// If `current_level` is finished, switch to the next level.
    pub fn next_level(&mut self) -> Result<(), NextLevelError> {
        let n = self.current_level.level.rank;
        let finished = self.current_level.is_finished();
        if finished && n < self.levels.len() {
            self.current_level = CurrentLevel::new(self.levels[n].clone());
            Ok(())
        } else if finished {
            Err(NextLevelError::EndOfCollection)
        } else {
            Err(NextLevelError::LevelNotFinished)
        }
    }

    /// Execute whatever command we get from the frontend.
    pub fn execute(&mut self, command: Command) {
        use Command::*;
        match command {
            Nothing => {}
            Move(dir) => {
                let _ = self.current_level.try_move(dir);
            }
            MoveAsFarAsPossible(dir, MayPushCrate(b)) => {
                let _ = self.current_level.move_until(dir, b);
            }
            MoveToPosition(pos, MayPushCrate(b)) => {
                let _ = self.current_level.move_to(pos, b);
            }
            Undo => self.current_level.undo(),
            Redo => self.current_level.redo(),
            NextLevel => {
                let _ = self.next_level();
            }
            PreviousLevel => unimplemented!(),
            LoadCollection(name) => {
                error!("Loading level collection {} is not implemented!", name);
            }
        }
    }

    pub fn current_direction(&self) -> Direction {
        self.current_level.current_direction()
    }
}

pub enum NextLevelError {
    LevelNotFinished,
    EndOfCollection,
}
