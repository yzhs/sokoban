use std::fmt;

use direction::Direction;

/// This structure contains everything needed to do or undo a Sokoban move.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    /// Was a crate moved?
    pub moves_crate: bool,

    /// Where was the move directed?
    pub direction: Direction,
}

impl Move {
    pub fn to_char(&self) -> char {
        if self.moves_crate {
            match self.direction {
                Direction::Left => 'L',
                Direction::Right => 'R',
                Direction::Up => 'U',
                Direction::Down => 'D',
            }
        } else {
            match self.direction {
                Direction::Left => 'l',
                Direction::Right => 'r',
                Direction::Up => 'u',
                Direction::Down => 'd',
            }
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}
