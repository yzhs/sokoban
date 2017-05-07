use std::io;
use std::io::Read;
use std::fs::File;
use std::path::Path;


use level::*;
use util::*;

#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub current_level: usize,
    pub levels: Vec<Level>,
}

impl Collection {
    pub fn load(name: &str) -> Result<Collection, io::Error> {
        let assets_path: &Path = ASSETS_PATH.as_ref();
        let mut level_path = assets_path.to_path_buf();
        level_path.push("levels");
        level_path.push(name);
        level_path.set_extension("lvl");

        let mut level_file = File::open(level_path)?;
        let mut content = "".to_string();
        level_file.read_to_string(&mut content)?;
        let level_strings: Vec<_> = content.split("\n\n").collect();
        let name = level_strings[0];

        Ok(Collection {
               name: name.to_string(),
               current_level: 0,
               levels: level_strings[1..]
                   .iter()
                   .enumerate()
                   .map(|(i, l)| Level::parse(i, l))
                   .collect(),
           })
    }

    pub fn level(&self, n: usize) -> Level {
        self.levels[n].clone()
    }
}
