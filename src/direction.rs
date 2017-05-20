use position::Position;

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
pub const DIRECTIONS: [Direction; 4] = [Direction::Left,
                                        Direction::Right,
                                        Direction::Up,
                                        Direction::Down];

/// Find out in which direction you have to move to get from `from` to `to`, if there is such a
/// direction. If both positions are the same, return `Err(None)`, if the two positions are neither
/// in the same row nor the same column, return `Err(Some(to))`. Otherwise, return `Ok(dirction)`.
pub fn direction(from: Position, to: Position) -> Result<Direction, Option<Position>> {
    // TODO better errors?
    use direction::Direction::*;
    let (dx, dy) = to - from;
    if dx == 0 && dy == 0 {
        Err(None)
    } else if dx == 0 && dy != 0 {
        Ok(if dy < 0 { Up } else { Down })
    } else if dx != 0 && dy == 0 {
        Ok(if dx < 0 { Left } else { Right })
    } else {
        Err(Some(to))
    }
}
