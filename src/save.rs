//! On-disc structures for storing which levels have been solved and the best solutions so far.

use std::convert::TryFrom;
use std::fs::File;
use std::path::PathBuf;
use std::cmp::Ordering;

use level::*;

/// One particular solution of a level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    number_of_moves: usize,
    number_of_pushes: usize,
    steps: String,
}

impl Solution {
    pub fn min_moves(&self, other: Solution) -> Self {
        match self.number_of_moves.cmp(&other.number_of_moves) {
            Ordering::Less => self.clone(),
            Ordering::Equal => {
                if self.number_of_pushes <= other.number_of_pushes {
                    self.clone()
                } else {
                    other
                }
            }
            Ordering::Greater => other,
        }
    }

    pub fn min_pushes(&self, other: Solution) -> Self {
        match self.number_of_pushes.cmp(&other.number_of_pushes) {
            Ordering::Less => self.clone(),
            Ordering::Equal => {
                if self.number_of_moves <= other.number_of_moves {
                    self.clone()
                } else {
                    other
                }
            }
            Ordering::Greater => other,
        }
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
    Started(Level),
    Finished {
        least_moves: Solution,
        least_pushes: Solution,
    },
}

impl LevelState {
    pub fn new(solution: Solution) -> Self {
        LevelState::Finished {
            least_moves: solution.clone(),
            least_pushes: solution,
        }
    }

    pub fn min(&self, other: Solution) -> Self {
        use self::LevelState::*;
        match *self {
            Started(_) => LevelState::new(other),
            Finished {
                ref least_moves,
                ref least_pushes,
            } => {
                Finished {
                    least_moves: least_moves.min_moves(other.clone()),
                    least_pushes: least_pushes.min_pushes(other),
                }
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        if let LevelState::Started(_) = *self {
            false
        } else {
            true
        }
    }
}

impl<'a> From<&'a Level> for LevelState {
    fn from(lvl: &'a Level) -> Self {
        if lvl.is_finished() {
            let soln = Solution::try_from(lvl).unwrap();
            LevelState::new(soln)
        } else {
            LevelState::Started(lvl.clone())
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
    pub fn new(name: &str) -> Self {
        CollectionState {
            name: name.to_string(),
            collection_solved: false,
            levels: vec![],
        }
    }

    pub fn load(name: &str) -> Self {
        let mut path = PathBuf::new();
        path.push("/home/yzhs/.local/share/sokoban");
        path.push(name);
        path.set_extension("json");
        match File::open(path) {
            Ok(file) => {
                ::serde_json::from_reader(file).unwrap_or_else(|_| CollectionState::new(name))
            }
            _ => CollectionState::new(name),
        }
    }

    pub fn len(&self) -> usize {
        self.levels.len()
    }

    pub fn update(&mut self, index: usize, level_state: LevelState) {
        if index >= self.len() {
            self.levels.push(level_state);
        } else {
            use self::LevelState::*;
            let ls_old = self.levels[index].clone();
            match ls_old {
                Started(_) => self.levels[index] = level_state,
                Finished {
                    least_moves: ref lm_old,
                    least_pushes: ref lp_old,
                } => {
                    if let Finished {
                               least_moves: lm,
                               least_pushes: lp,
                           } = level_state {
                        self.levels[index] = Finished {
                            least_moves: lm_old.min_moves(lm),
                            least_pushes: lp_old.min_pushes(lp),
                        };
                    }
                }
            }
        }
    }

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
