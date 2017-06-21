use std::ops::Sub;

use direction::Direction;

/// A position in a Sokoban level given as (x,y) coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: isize,
    pub y: isize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Position {
            x: x as isize,
            y: y as isize,
        }
    }

    pub fn above(&self) -> Self {
        Position {
            x: self.x,
            y: self.y - 1,
        }
    }

    pub fn below(&self) -> Self {
        Position {
            x: self.x,
            y: self.y + 1,
        }
    }

    pub fn left(&self) -> Self {
        Position {
            x: self.x - 1,
            y: self.y,
        }
    }

    pub fn right(&self) -> Self {
        Position {
            x: self.x + 1,
            y: self.y,
        }
    }

    pub fn from_index(index: usize, columns: usize) -> Self {
        Position {
            x: (index % columns) as isize,
            y: (index / columns) as isize,
        }
    }

    pub fn to_index(&self, columns: usize) -> usize {
        self.x as usize + self.y as usize * columns
    }

    /// Return the neighbouring Position in the given direction.
    pub fn neighbour(&self, direction: Direction) -> Self {
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
