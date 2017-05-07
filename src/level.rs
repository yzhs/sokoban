use std::convert::TryFrom;
use std::fmt;

use cell::*;

#[derive(Debug, Clone)]
pub struct Level {
    level_number: usize,
    pub width: usize,
    pub height: usize,

    /// width * height cells in row-major order
    pub background: Vec<Background>,
    pub foreground: Vec<Foreground>,
}

impl Level {
    /// Parse the ASCII representation of a level.
    pub fn parse(num: usize, string: &str) -> Level {
        let lines: Vec<_> = string.split("\n").collect();
        let height = lines.len();
        let width = lines.iter().map(|x| x.len()).max().unwrap();
        let mut background = vec![Background::Empty; width * height];
        let mut foreground = vec![Foreground::None; width * height];

        for (i, line) in lines.iter().enumerate() {
            let mut inside = false;
            for (j, chr) in line.chars().enumerate() {
                let cell = Cell::try_from(chr)
                    .expect(format!("Invalid character '{}' in line {}, column {}.", chr, i, j)
                                .as_ref());
                if !inside && cell.background == Background::Wall {
                    inside = true;
                }

                let index = i * width + j;
                background[index] = cell.background;
                foreground[index] = cell.foreground;
                if inside && cell.background == Background::Empty && i == 0 ||
                   background[index - width] != Background::Empty {
                    background[index] = Background::Floor;
                }
            }
        }

        Level {
            level_number: num + 1,
            width: width,
            height: height,
            background: background,
            foreground: foreground,
        }
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
