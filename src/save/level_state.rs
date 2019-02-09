use std::convert::TryFrom;

use super::solution::*;
use crate::current_level::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LevelState {
    /// The level has not been finished.
    Started {
        number_of_moves: usize,
        moves: String,
    },

    /// The level has been finished.
    Finished {
        /// The solution using the least number of moves.
        least_moves: Solution,

        /// The solution using the least number of pushes.
        least_pushes: Solution,
    },
}

impl LevelState {
    pub fn new_solved(solution: Solution) -> Self {
        LevelState::Finished {
            least_moves: solution.clone(),
            least_pushes: solution,
        }
    }

    pub fn new_unsolved(level: &CurrentLevel) -> Self {
        LevelState::Started {
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
}

impl<'a> From<&'a CurrentLevel> for LevelState {
    fn from(lvl: &'a CurrentLevel) -> Self {
        if lvl.is_finished() {
            let soln = Solution::try_from(lvl).unwrap();
            LevelState::new_solved(soln)
        } else {
            LevelState::new_unsolved(lvl)
        }
    }
}
