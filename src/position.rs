use std::ops::Sub;

use direction::Direction;

/// A position in a Sokoban level given as (x,y) coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: isize,
    pub y: isize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Position {
        Position {
            x: x as isize,
            y: y as isize,
        }
    }

    /// Return the neighbouring Position in the given direction.
    pub fn neighbour(&self, direction: Direction) -> Position {
        use super::Direction::*;
        let (x, y) = match direction {
            Left => (self.x - 1, self.y),
            Right => (self.x + 1, self.y),
            Up => (self.x, self.y - 1),
            Down => (self.x, self.y + 1),
        };
        Position { x, y }
    }
}

impl Sub for Position {
    type Output = (isize, isize);
    fn sub(self, other: Position) -> (isize, isize) {
        (self.x - other.x, self.y - other.y)
    }
}
