use std::io::Read;
use std::fs::File;
use std::path::Path;


use backend::level::*;
use backend::util::*;

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

    pub fn level(&self, n: usize) -> Level {
        self.levels[n].clone()
    }

    /// If `current_level` is finished, switch to the next level.
    pub fn next_level(&mut self) {
        let n = self.current_level.level.level_number;
        if self.current_level.is_finished() && n < self.levels.len() {
            self.current_level = CurrentLevel::new(self.levels[n].clone());
        } else if self.current_level.is_finished() {
            error!("Reached the end of the current collection.");
        } else {
            error!("Current level is not finished!");
        }
    }
}
