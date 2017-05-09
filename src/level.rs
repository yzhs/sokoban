use std::convert::TryFrom;
use std::fmt;

use cell::*;
use direction::*;
use move_::*;
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

    empty_goals: usize,
    worker_position: (usize, usize),
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
                        return Err(SokobanError::TwoWorkers(num + 1));
                    }
                    worker_position = (i, j);
                    found_worker = true;
                }
            }
        }

        if !found_worker {
            return Err(SokobanError::NoWorker(num + 1));
        } else if goals_minus_crates != 0 {
            return Err(SokobanError::CratesGoalsMismatch(num + 1, goals_minus_crates));
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
               background,
               foreground,

               empty_goals,
               worker_position,
           })
    }

    /// Try to move in the given direction. Return an error if that is not possile.
    pub fn try_move(&mut self, direction: Direction) -> Result<(), ()> {
        use self::Direction::*;

        let (x, y) = (self.worker_position.0 as isize, self.worker_position.1 as isize);
        let (dx, dy): (isize, isize) = match direction {
            Left => (-1, 0),
            Right => (1, 0),
            Up => (0, -1),
            Down => (0, 1),
        };
        let next = (x + dx, y + dy);
        let next_index = next.0 as usize + next.1 as usize * self.width;
        let next_but_one = (x + 2 * dx, y + 2 * dy);
        let moves_crate = if self.is_empty(next) {
            // Move to empty cell
            false
        } else if self.is_crate(next) && self.is_empty(next_but_one) {
            // Push crate into empty next cell
            let next_but_one_index = next_but_one.0 as usize + next_but_one.1 as usize * self.width;
            self.foreground[next_but_one_index] = Foreground::Crate;
            self.foreground[next_index] = Foreground::None;
            true
        } else {
            return Err(());
        };

        // Move worker to new position
        let index = x as usize + y as usize * self.width;
        self.foreground[next_index] = Foreground::Worker;
        self.foreground[index] = Foreground::None;
        self.worker_position = (next.0 as usize, next.1 as usize);
        // TODO check how this affects the number of crates on goals

        Ok(())
    }

    /// Is there a crate at the given position?
    fn is_crate(&self, pos: (isize, isize)) -> bool {
        // Check bounds
        if pos.0 < 0 || pos.1 < 0 || pos.0 as usize >= self.width || pos.1 as usize >= self.height {
            return false;
        }

        // Check the cell itself
        let index = pos.0 as usize + self.width * pos.1 as usize;
        self.foreground[index] == Foreground::Crate
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_empty(&self, pos: (isize, isize)) -> bool {
        use self::Background::*;
        use self::Foreground::*;

        // Check bounds
        if pos.0 < 0 || pos.1 < 0 || pos.0 as usize >= self.width || pos.1 as usize >= self.height {
            return false;
        }

        // Check the cell itself
        let index = pos.0 as usize + self.width * pos.1 as usize;
        match (self.background[index], self.foreground[index]) {
            (Floor, None) | (Goal, None) => true,
            _ => false,
        }
    }

    /// Check whether the given level is completed, i.e. every goal has a crate on it, and every
    /// crate is on a goal.
    pub fn is_finished(&self) -> bool {
        self.empty_goals == 0
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
        assert_eq!(res.unwrap_err().to_string(), "TwoWorkers(1)");
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
        assert_eq!(res.unwrap_err().to_string(), "NoWorker(1)");
    }

    #[test]
    fn test_trivial_move_1() {
        use self::Direction::*;
        let mut lvl = Level::parse(0,
                                   "####\n\
                                       #@ #\n\
                                       ####\n")
                .unwrap();
        assert_eq!(lvl.worker_position.0, 1);
        assert_eq!(lvl.worker_position.1, 1);

        assert!(lvl.is_empty((2, 1)));
        assert!(!lvl.is_empty((0, 1)));
        for y in 0..3 {
            for x in 0..4 {
                assert!(!lvl.is_crate((x, y)));
            }
        }

        assert!(lvl.try_move(Right).is_ok());
        assert!(lvl.try_move(Left).is_ok());
        assert!(lvl.try_move(Left).is_err());
        assert!(lvl.try_move(Up).is_err());
        assert!(lvl.try_move(Down).is_err());
    }

    #[test]
    fn test_trivial_move_2() {
        use self::Direction::*;
        let mut lvl = Level::parse(0,
                                   "#######\n\
                                       #.$@$.#\n\
                                       #######\n")
                .unwrap();
        assert_eq!(lvl.worker_position.0, 3);
        assert_eq!(lvl.worker_position.1, 1);
        assert!(lvl.try_move(Right).is_ok());
        assert!(lvl.try_move(Left).is_ok());
        assert!(lvl.try_move(Up).is_err());
        assert!(lvl.try_move(Down).is_err());
    }
}
