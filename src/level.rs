use std::convert::TryFrom;
use std::fmt;

use cell::*;
use util::*;

#[derive(Debug, Clone)]
pub struct Level {
    level_number: usize,
    pub width: usize,
    pub height: usize,

    /// width * height cells backgrounds in row-major order
    pub background: Vec<Background>,

    /// width * height cell array of worker and crates in row-major order
    pub foreground: Vec<Foreground>,
}

impl Level {
    /// Parse the ASCII representation of a level.
    pub fn parse(num: usize, string: &str) -> Result<Level, SokobanError> {
        let lines: Vec<_> = string.split("\n").collect();
        let height = lines.len();
        let width = lines.iter().map(|x| x.len()).max().unwrap();
        let mut background = vec![Background::Empty; width * height];
        let mut foreground = vec![Foreground::None; width * height];

        let mut goals_minus_crates = 0i32;

        for (i, line) in lines.iter().enumerate() {
            let mut inside = false;
            for (j, chr) in line.chars().enumerate() {
                let cell = Cell::try_from(chr)
                    .expect(format!("Invalid character '{}' in line {}, column {}.", chr, i, j)
                                .as_ref());
                let index = i * width + j;
                background[index] = cell.background;
                foreground[index] = cell.foreground;

                // Make sure there are exactly the same number of crates and goals.
                if cell.background == Background::Goal {
                    goals_minus_crates += 1;
                }
                if cell.foreground == Foreground::Crate {
                    goals_minus_crates -= 1;
                }

                // Try to figure out whether a given cell is inside the walls.
                if !inside && cell.background == Background::Wall {
                    inside = true;
                }

                if inside && cell.background == Background::Empty &&
                   (index < width || background[index - width] != Background::Empty) {
                    background[index] = Background::Floor;
                }
            }
        }

        if goals_minus_crates != 0 {
            return Err(SokobanError::CratesGoalsMismatch(num+1, goals_minus_crates));
        }

        }
        Ok(Level {
               level_number: num + 1, // The first level is level 1
               width,
               height,
               background,
               foreground,
           })
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.height {
            if i != 0 {
                write!(f, "\n")?;
            }
            for j in 0..self.width {
                let index = i * self.width + j;
                let foreground = self.foreground[index];
                let background = self.background[index];
                write!(f,
                       "{}",
                       Cell {
                               foreground,
                               background,
                           }
                           .to_char())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_crate_missing() {
        let s = ".*.*.";
        let res = Level::parse(0, s);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "CratesGoalsMismatch(1, 3)");
    }
}
