use std::convert::TryFrom;

use super::solution::*;
use level::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LevelState {
    /// The level has not been finished.
    Started {
        #[serde(default)]
        rank: usize,
        number_of_moves: usize,
        moves: String,
    },

    /// The level has been finished.
    Finished {
        #[serde(default)]
        rank: usize,
        /// The solution using the least number of moves.
        least_moves: Solution,

        /// The solution using the least number of pushes.
        least_pushes: Solution,
    },
}

impl LevelState {
    pub fn new_solved(rank: usize, solution: Solution) -> Self {
        LevelState::Finished {
            rank,
            least_moves: solution.clone(),
            least_pushes: solution,
        }
    }

    pub fn new_unsolved(level: &Level) -> Self {
        LevelState::Started {
            rank: level.rank(),
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

    pub fn rank(&self) -> usize {
        match *self {
            LevelState::Started { rank, .. } | LevelState::Finished { rank, .. } => rank,
        }
    }

    pub fn set_rank(&mut self, new_rank: usize) {
        match *self {
            LevelState::Started { ref mut rank, .. }
            | LevelState::Finished { ref mut rank, .. } => *rank = new_rank,
        }
    }
}

impl<'a> From<&'a Level> for LevelState {
    fn from(lvl: &'a Level) -> Self {
        if lvl.is_finished() {
            let soln = Solution::try_from(lvl).unwrap();
            LevelState::new_solved(lvl.rank(), soln)
        } else {
            LevelState::new_unsolved(lvl)
        }
    }
}
