use std::cmp::Ordering;
use std::convert::TryFrom;

use crate::level::*;

/// One particular solution of a level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    number_of_moves: usize,
    number_of_pushes: usize,
    steps: String,
}

impl Solution {
    /// Return a copy of either `self` or `other` with the smallest number of *worker* movements.
    pub fn min_moves(&self, other: &Solution) -> Self {
        match self.number_of_moves.cmp(&other.number_of_moves) {
            Ordering::Less => self.clone(),
            Ordering::Equal if self.number_of_pushes <= other.number_of_pushes => self.clone(),
            _ => other.clone(),
        }
    }

    /// Return a copy of either `self` or `other` with the smallest number of *crate* movements.
    pub fn min_pushes(&self, other: &Solution) -> Self {
        match self.number_of_pushes.cmp(&other.number_of_pushes) {
            Ordering::Less => self.clone(),
            Ordering::Equal if self.number_of_moves <= other.number_of_moves => self.clone(),
            _ => other.clone(),
        }
    }

    /// Is `self` a solution with less moves than `other`?
    pub fn less_moves(&self, other: &Solution) -> bool {
        self.number_of_moves < other.number_of_moves
    }

    /// Is `self` a solution with less pushes than `other`?
    pub fn less_pushes(&self, other: &Solution) -> bool {
        self.number_of_pushes < other.number_of_pushes
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
