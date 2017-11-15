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

    /// Describe a move using one character signifying its direction. The character is upper case
    /// if and only if `self.moves_crate` is true.
    pub fn to_char(&self) -> char {
        use std::ascii::AsciiExt;
        let mut c = match self.direction {
            Direction::Left => 'l',
            Direction::Right => 'r',
            Direction::Up => 'u',
            Direction::Down => 'd',
        };
        if self.moves_crate {
            c.make_ascii_uppercase();
        }
        c
    }
}

/// Parse a string representation of moves.
pub fn parse(s: &str) -> Result<Vec<Move>, char> {
    s.chars().map(Move::try_from).collect::<Result<Vec<_>, _>>()
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

impl TryFrom<char> for Move {
    type Error = char;

    fn try_from(c: char) -> Result<Move, char> {
        use std::ascii::AsciiExt;
        use Direction::*;
        let dir = match c {
            'l' | 'L' => Left,
            'r' | 'R' => Right,
            'u' | 'U' => Up,
            'd' | 'D' => Down,
            _ => return Err(c),
        };
        let push = c.is_ascii_uppercase();
        Ok(Move::new(dir, push))
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

    #[test]
    fn parse_str() {
        let s = "UldrdddDddlLrrRRuLulLLUUdrdlduUDLR";
        let moves = parse(s).unwrap();
        let s2: String = moves.into_iter().map(|x| x.to_char()).collect();
        assert_eq!(s, s2);
    }
}
