use std::fmt;

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};

use crate::position::Position;

/// Any of the directions needed for Sokoban.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// Return the opposite direction. This is used when undoing a move.
    pub fn reverse(self) -> Self {
        use self::Direction::*;
        match self {
            Left => Right,
            Right => Left,
            Up => Down,
            Down => Up,
        }
    }
}

/// All directions
pub const DIRECTIONS: [Direction; 4] = [
    Direction::Left,
    Direction::Right,
    Direction::Up,
    Direction::Down,
];

#[derive(Debug, PartialEq)]
pub enum DirectionResult {
    SamePosition,
    Neighbour { direction: Direction },
    Other,
}

/// Find out in which direction you have to move to get from `from` to `to`, if there is such a
/// direction. If both positions are the same, return `Err(None)`, if the two positions are neither
/// in the same row nor the same column, return `Err(Some(to))`. Otherwise, return `Ok(dirction)`.
pub fn direction(from: Position, to: Position) -> DirectionResult {
    use crate::direction::Direction::*;

    match to - from {
        (0, 0) => DirectionResult::SamePosition,
        (0, dy) => DirectionResult::Neighbour {
            direction: if dy < 0 { Up } else { Down },
        },
        (dx, 0) => DirectionResult::Neighbour {
            direction: if dx < 0 { Left } else { Right },
        },
        _ => DirectionResult::Other,
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Direction::*;
        write!(
            f,
            "{}",
            match *self {
                Left => 'l',
                Right => 'r',
                Up => 'u',
                Down => 'd',
            }
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_positions() {
        let pos0 = Position::new(0, 42);
        for &dir in DIRECTIONS.iter() {
            let pos1 = pos0.neighbour(dir);
            assert_eq!(
                direction(pos0, pos1),
                DirectionResult::Neighbour { direction: dir }
            );
            assert_eq!(pos1.neighbour(dir.reverse()), pos0);
        }
        assert_eq!(direction(pos0, pos0), DirectionResult::SamePosition);
        assert_eq!(direction(pos0.left().above(), pos0), DirectionResult::Other);
    }
}

#[cfg(test)]
impl Arbitrary for Direction {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        DIRECTIONS[g.next_u32() as usize % 4_usize]
    }
}
