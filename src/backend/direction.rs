use backend::position::Position;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
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

pub const DIRECTIONS: [Direction; 4] = [Direction::Left,
                                        Direction::Right,
                                        Direction::Up,
                                        Direction::Down];

pub fn direction(from: Position, to: Position) -> Result<Direction, Option<Position>> {
    // TODO better errors?
    use backend::direction::Direction::*;
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
