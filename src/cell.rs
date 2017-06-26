use std::convert::TryFrom;


/// Static part of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Background {
    Empty,
    Wall,
    Floor,
    Goal,
}

impl Background {
    pub fn is_wall(self) -> bool {
        match self {
            Background::Wall => true,
            _ => false,
        }
    }
}

/// Dynamic part of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Foreground {
    None,
    Worker,
    Crate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub background: Background,
    pub foreground: Foreground,
}


#[derive(Debug, Copy, Clone)]
pub struct TryFromCellError(());

impl TryFromCellError {
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        "invalid character"
    }
}

impl TryFrom<char> for Cell {
    type Error = TryFromCellError;
    /// Try to parse a given character as part of a level description.
    fn try_from(c: char) -> Result<Cell, TryFromCellError> {
        use Background::*;
        use Foreground::*;
        match c {
            ' ' => {
                Ok(Cell {
                       background: Empty,
                       foreground: Foreground::None,
                   })
            }
            '#' => {
                Ok(Cell {
                       background: Wall,
                       foreground: Foreground::None,
                   })
            }
            '.' => {
                Ok(Cell {
                       background: Goal,
                       foreground: Foreground::None,
                   })
            }
            '@' => {
                Ok(Cell {
                       background: Floor,
                       foreground: Worker,
                   })
            }
            '*' => {
                Ok(Cell {
                       background: Goal,
                       foreground: Crate,
                   })
            }
            '$' => {
                Ok(Cell {
                       background: Floor,
                       foreground: Crate,
                   })
            }
            '+' => {
                Ok(Cell {
                       background: Goal,
                       foreground: Worker,
                   })
            }
            _ => Err(TryFromCellError(())),
        }
    }
}

impl Cell {
    /// Given a Cell, return the character representing it in the on-disc format.
    pub fn to_char(self) -> char {
        use Background::*;
        use Foreground::*;
        match self {
            Cell {
                background: Empty,
                foreground: Foreground::None,
            } |
            Cell {
                background: Floor,
                foreground: Foreground::None,
            } => ' ',
            Cell {
                background: Wall,
                foreground: Foreground::None,
            } => '#',
            Cell {
                background: Goal,
                foreground: Foreground::None,
            } => '.',
            Cell {
                background: Floor,
                foreground: Worker,
            } => '@',
            Cell {
                background: Goal,
                foreground: Crate,
            } => '*',
            Cell {
                background: Floor,
                foreground: Crate,
            } => '$',
            Cell {
                background: Goal,
                foreground: Worker,
            } => '+',
            _ => panic!(format!("Invalid cell: {:?}", self)),
        }
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_from_char_to_char() {
        let s = " #. @@*$+ +#.";
        assert_eq!(s,
                   s.chars()
                       .map(|c| Cell::try_from(c).unwrap().to_char())
                       .collect::<String>());
    }

    #[test]
    fn test_only_valid_chars() {
        let s = "abcdefghijlmopqrstuvwxyzABCDEFLMNOPTUVW24567890\\/_-αμ∈∩\n\r\t\"'<>[](){}";
        for c in s.chars() {
            assert!(Cell::try_from(c).is_err());
        }
        for c in " #.@*$+".chars() {
            assert!(Cell::try_from(c).is_ok());
        }
    }
}
