use std::convert::TryFrom;
use std::fmt;

use cell::*;
use util::*;

#[derive(Debug, Clone)]
pub struct Level {
    level_number: usize,
    pub width: usize,
    pub height: usize,

    empty_goals: usize,
    worker_position: (usize, usize),

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

        let mut found_worker = false;
        let mut worker_position = (0, 0);
        let mut empty_goals = 0;
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

                // Count goals still to be filled.
                if background[index] == Background::Goal && foreground[index] != Foreground::Crate {
                    empty_goals += 1;
                }

                // Find the initial worker position.
                if foreground[index] == Foreground::Worker {
                    if found_worker {
                        return Err(SokobanError::TwoWorkers);
                    }
                    worker_position = (i, j);
                    found_worker = true;
                }
            }
        }

        if !found_worker {
            return Err(SokobanError::NoWorker);
        }

        if goals_minus_crates != 0 {
            return Err(SokobanError::CratesGoalsMismatch(num+1, goals_minus_crates));
        }

        // Fix the mistakes of the above heuristic for detecting which cells are on the inside.
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..height {
                for j in 0..width {
                    let index = i * width + j;
                    if background[index] != Background::Floor {
                        continue;
                    }

                    // A non-wall cell next to an outside cell has to be on the outside itself.
                    if index > width && background[index - width] == Background::Empty ||
                       i < height - 1 && background[index + width] == Background::Empty ||
                       j < width - 1 && background[index + 1] == Background::Empty {
                        background[index] = Background::Empty;
                        changed = true;
                    }
                }
            }
        }

        Ok(Level {
               level_number: num + 1, // The first level is level 1
               width,
               height,
               empty_goals,
               worker_position,
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
        let s = "@.*.*.";
        let res = Level::parse(0, s);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "CratesGoalsMismatch(1, 3)");
    }

    #[test]
    fn test_two_workers() {
        let s = "############\n\
                 #..  #     ###\n\
                 #.. @# $  $  #\n\
                 #..  #$####  #\n\
                 #..    @ ##  #\n\
                 #..  # #  $ ##\n\
                 ###### ##$ $ #\n\
                   # $  $ $ $ #\n\
                   #    #     #\n\
                   ############";
        let res = Level::parse(0, s);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "TwoWorkers");
    }

    #[test]
    fn test_no_workers() {
        let s = "############\n\
                 #..  #     ###\n\
                 #..  # $  $  #\n\
                 #..  #$####  #\n\
                 #..    # ##  #\n\
                 #..  # #  $ ##\n\
                 ###### ##$ $ #\n\
                   # $  $ $ $ #\n\
                   #    #     #\n\
                   ############";
        let res = Level::parse(0, s);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "NoWorker");
    }
}
