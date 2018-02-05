use std::fs;
use std::path::PathBuf;

use collection::*;
use command::*;
use direction::Direction;
use level::Level;
use position::Position;
use util::{SokobanError, ASSETS};

#[derive(Debug)]
pub struct Game {
    pub name: String,
    collection: Collection,
}

impl Game {
    pub fn new(name: &str) -> Result<Self, SokobanError> {
        Ok(Game {
            name: name.into(),
            collection: Collection::parse(name, true)?,
        })
    }

    /// Load a collection by name.
    pub fn set_collection(&mut self, name: &str) -> Result<(), SokobanError> {
        self.name = name.into();
        self.collection = Collection::parse(name, true)?;
        Ok(())
    }

    /// Execute a command from the front end. Load new collections or pass control to
    /// `Collection::execute`.
    pub fn execute(&mut self, cmd: &Command) -> Vec<Response> {
        if let Command::LoadCollection(ref name) = *cmd {
            error!("Loading level collection {}.", name);
            self.set_collection(name).unwrap();
            vec![Response::NewLevel(self.collection.current_level.rank)]
        } else {
            self.collection.execute(cmd)
        }
    }

    // Access data concerning the current level
    /// The current level
    pub fn current_level(&self) -> &Level {
        &self.collection.current_level
    }

    /// The rank of the current level in the current collection.
    pub fn rank(&self) -> usize {
        self.collection.current_level.rank
    }

    /// Is the current level the last one of this collection?
    pub fn end_of_collection(&self) -> bool {
        self.collection.end_of_collection()
    }

    /// The number of columns of the current level.
    pub fn columns(&self) -> usize {
        self.collection.current_level.columns()
    }

    /// The number of rows of the current level.
    pub fn rows(&self) -> usize {
        self.collection.current_level.rows()
    }

    /// Get an ordered list of the crates’ positions where the id of a crate is its index in the
    /// list.
    pub fn crate_positions(&self) -> Vec<Position> {
        let mut crates: Vec<_> = self.current_level().crates.iter().collect();
        crates.sort_by_key(|&(_pos, id)| id);
        crates.into_iter().map(|(&pos, _id)| pos).collect()
    }

    /// Where is the worker?
    pub fn worker_position(&self) -> Position {
        self.collection.current_level.worker_position
    }

    /// What is the direction of the worker’s last move?
    pub fn worker_direction(&self) -> Direction {
        self.collection.worker_direction()
    }

    /// The number of moves performed since starting to solve this level.
    pub fn number_of_moves(&self) -> usize {
        self.collection.current_level.number_of_moves()
    }

    /// The number of pushes performed since starting to solve this level.
    pub fn number_of_pushes(&self) -> usize {
        self.collection.current_level.number_of_pushes()
    }

    /// The collections full name
    pub fn name(&self) -> &str {
        self.collection.name.as_ref()
    }
}

fn file_stem(p: &::std::path::PathBuf) -> &str {
    p.file_stem().unwrap().to_str().unwrap()
}

pub fn print_collections_table() {
    use ansi_term::Colour::{Blue, Green, White, Yellow};

    println!(
        " {}               {}",
        Yellow.bold().paint("File name"),
        Yellow.bold().paint("Collection name")
    );
    println!("{0}{0}{0}{0}{0}", "----------------");

    // Find all level set files
    let mut paths: Vec<PathBuf> = fs::read_dir(ASSETS.join("levels"))
        .unwrap()
        .map(|x| x.unwrap().path().to_owned())
        .collect();
    paths.sort_by(|x, y| ::natord::compare(file_stem(x), file_stem(y)));

    for path in paths {
        if let Some(ext) = path.extension() {
            use std::ffi::OsStr;
            if ext == OsStr::new("lvl") || ext == OsStr::new("slc") {
                let name = path.file_stem().and_then(|x| x.to_str()).unwrap();
                let collection = Collection::parse(name, false).unwrap();

                let padded_short_name = format!("{:<24}", name);
                let padded_full_name = format!("{:<36}", collection.name);

                if collection.is_solved() {
                    println!(
                        " {}{}{:>10} {}",
                        Green.paint(padded_short_name),
                        Green.bold().paint(padded_full_name),
                        "",
                        Green.paint("done")
                    );
                } else {
                    let num_solved = collection.number_of_solved_levels();
                    let solved = if num_solved == 0 {
                        White.paint("solved")
                    } else {
                        Blue.paint("solved")
                    };
                    println!(
                        " {}{}{:>10} {}",
                        padded_short_name,
                        White.bold().paint(padded_full_name),
                        format!("{}/{}", num_solved, collection.number_of_levels()),
                        solved
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_game() -> Game {
        const LARGE_EMPTY_LEVEL: &str = r#"
#########################################
#                                    #$.#
#                                    ####
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                 @                  #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
######################################
"#;
        const NAME: &str = "Test";
        let lvl = Level::parse(0, LARGE_EMPTY_LEVEL).unwrap();
        let collection = Collection::from_levels(NAME, &[lvl.clone()]);
        Game {
            name: "LARGE_EMPTY_LEVEL".into(),
            collection,
        }
    }

    #[quickcheck]
    fn prop_move_undo(mut move_dirs: Vec<Direction>) -> bool {
        let mut game = create_game();
        let lvl = game.current_level().clone();
        move_dirs.truncate(10);

        let num_moves = move_dirs.len();
        for dir in move_dirs {
            game.execute(&Command::Move(dir));
        }
        for _ in 0..num_moves {
            game.execute(&Command::Undo);
        }

        let current_lvl = game.current_level();
        current_lvl.worker_position == lvl.worker_position
            && current_lvl.number_of_moves() == lvl.number_of_moves()
    }
}
