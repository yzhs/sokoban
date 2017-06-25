use std::convert::TryFrom;
use std::fmt;

use direction::Direction;

/// This structure contains everything needed to do or undo a Sokoban move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Move {
    /// Was a crate moved?
    pub moves_crate: bool,

    /// Where was the move directed?
    pub direction: Direction,
}

impl Move {
    pub fn new(direction: Direction, moves_crate: bool) -> Self {
        Move {
            moves_crate,
            direction,
        }
    }

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

pub fn parse(s: &str) -> Result<Vec<Move>, char> {
    s.chars()
        .map(Move::try_from)
        .collect::<Result<Vec<_>, _>>()
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

impl TryFrom<char> for Move {
    type Error = char;

    fn try_from(c: char) -> Result<Move, char> {
        use Direction::*;
        Ok(match c {
               'l' => Move::new(Left, false),
               'L' => Move::new(Left, true),
               'r' => Move::new(Right, false),
               'R' => Move::new(Right, true),
               'u' => Move::new(Up, false),
               'U' => Move::new(Up, true),
               'd' => Move::new(Down, false),
               'D' => Move::new(Down, true),
               _ => return Err(c),
           })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_from() {
        for &dir in &::direction::DIRECTIONS {
            let mv = Move::new(dir, true);
            assert_eq!(Ok(mv.clone()), Move::try_from(mv.to_char()));
            let mv = Move::new(dir, false);
            assert_eq!(Ok(mv.clone()), Move::try_from(mv.to_char()));
        }
    }

    #[test]
    fn invalid_char() {
        for chr in "abcefghijkmnopqstvwxyz".chars() {
            assert!(Move::try_from(chr).is_err());
        }
    }
}
