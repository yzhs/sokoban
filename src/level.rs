use std::convert::TryFrom;
use std::fmt;

use cell::*;
use direction::*;
use move_::*;
use util::*;
pub use position::*;


#[derive(Debug, Clone)]
pub struct Level {
    pub level_number: usize,
    pub width: usize,
    pub height: usize,

    /// width * height cells backgrounds in row-major order
    pub background: Vec<Background>,

    /// width * height cell array of worker and crates in row-major order
    pub foreground: Vec<Foreground>,

    empty_goals: usize,
    pub worker_position: Position,
}

#[derive(Debug, Clone)]
pub struct CurrentLevel {
    pub level: Level,

    /// The sequence of moves performed so far. Everything after the first moves_recorded moves is
    /// used to redo moves, i.e. undoing a previous undo operation.
    moves: Vec<Move>,

    /// This describes how many moves have to be performed to arrive at the current state.
    moves_recorded: usize,

    empty_goals: usize,
    pub worker_position: Position,
}


impl Level {
    /// Parse the ASCII representation of a level.
    pub fn parse(num: usize, string: &str) -> Result<Level, SokobanError> {
        let lines: Vec<_> = string.split('\n').collect();
        let height = lines.len();
        let width = lines.iter().map(|x| x.len()).max().unwrap();

        let mut found_worker = false;
        let mut worker_position = Position { x: 0, y: 0 };
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
                    worker_position = Position::new(j, i);
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
}

impl CurrentLevel {
    pub fn new(level: Level) -> CurrentLevel {
        CurrentLevel {
            moves: vec![],
            moves_recorded: 0,
            empty_goals: level.empty_goals,
            worker_position: level.worker_position,
            level,
        }
    }

    pub fn background(&self, x: isize, y: isize) -> &Background {
        &self.level.background[x as usize + y as usize * self.level.width]
    }

    pub fn foreground(&self, x: isize, y: isize) -> &Foreground {
        &self.level.foreground[x as usize + y as usize * self.level.width]
    }

    pub fn foreground_mut(&mut self, x: isize, y: isize) -> &mut Foreground {
        &mut self.level.foreground[(x as usize + y as usize * self.level.width)]
    }

    /// Try to find a shortest path from the workers current position to `to` and execute it if one
    /// exists.
    fn find_path(&mut self, to: Position) {
        info!("Finding path to {:?}…", to);
        unimplemented!();
    }

    /// Move the worker towards `to`. If may_push_crate is set, `to` must be in the same row or
    /// column as the worker. In that case, the worker moves to `to`
    pub fn move_to(&mut self, to: Position, may_push_crate: bool) {
        use self::Direction::*;

        let (dx, dy) = to - self.worker_position;

        let direction = if dx != 0 && dy != 0 {
            if may_push_crate {
                error!("Can only move in the same row or column while pushing crates");
            } else {
                self.find_path(to);
            }
            return;
        } else if dx == 0 && dy == 0 {
            // Already there
            return;
        } else if dx == 0 {
            if dy < 0 { Up } else { Down }
        } else {
            // dy == 0
            if dx < 0 { Left } else { Right }
        };

        while self.move_helper(direction, may_push_crate).is_ok() && self.worker_position != to {}
    }

    /// Try to move in the given direction. Return an error if that is not possile.
    pub fn try_move(&mut self, direction: Direction) -> Result<(), ()> {
        self.move_helper(direction, true)
    }

    pub fn move_until(&mut self, direction: Direction, may_push_crate: bool) {
        while self.move_helper(direction, may_push_crate).is_ok() {}
    }

    fn move_helper(&mut self, direction: Direction, may_push_crate: bool) -> Result<(), ()> {
        let next = self.worker_position.neighbour(direction);
        let next_but_one = next.neighbour(direction);

        let moves_crate = if self.is_empty(next) {
            // Move to empty cell
            false
        } else if self.is_crate(next) && self.is_empty(next_but_one) &&
                  may_push_crate {
            // Push crate into empty next cell
            let _ = self.move_object(next, direction, false);
            true
        } else {
            return Err(());
        };

        // Move worker to new position
        let pos = self.worker_position;
        let (worker_pos, _) = self.move_object(pos, direction, false);
        self.worker_position = worker_pos;

        // Bookkeeping for undo and printing a solution
        let current_move = Move {
            direction,
            moves_crate,
        };
        let n = self.moves_recorded;
        self.moves_recorded += 1;

        if n != self.moves.len() && self.moves[n] == current_move {
            // In this case, we are just redoing a move previously undone
        } else {
            if n != self.moves.len() {
                // Discard redo buffer as we are in a different state than before
                self.moves.truncate(n);
            }
            self.moves.push(current_move);
        }

        Ok(())
    }

    /// Is there a crate at the given position?
    fn is_crate(&self, pos: Position) -> bool {
        // Check bounds
        if pos.x < 0 || pos.y < 0 || pos.x as usize >= self.level.width ||
           pos.y as usize >= self.level.height {
            return false;
        }

        // Check the cell itself
        self.foreground(pos.x, pos.y) == &Foreground::Crate
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_empty(&self, pos: Position) -> bool {
        use self::Background::*;
        use self::Foreground::*;
        let (x, y) = (pos.x as isize, pos.y as isize);

        // Check bounds
        if pos.x < 0 || pos.y < 0 || x as usize >= self.level.width ||
           y as usize >= self.level.height {
            return false;
        }

        // Check the cell itself
        match (*self.background(x, y), *self.foreground(x, y)) {
            (Floor, None) | (Goal, None) => true,
            _ => false,
        }
    }

    /// Move whatever object is in the foreground at the given position in the given direction if
    /// undo is false, and in the opposite direction otherwise. Return the new position of that
    /// object, as well as the position of the object behind the original position. This is needed
    /// to move crates backwards when undoing a push.
    fn move_object(&mut self,
                   from: Position,
                   direction: Direction,
                   undo: bool)
                   -> (Position, Position) {
        let direction = if undo { direction.reverse() } else { direction };
        let new = from.neighbour(direction);

        // Make sure empty_goals is updated as needed.
        if self.foreground_mut(from.x, from.y) == &Foreground::Crate {
            if self.background(from.x, from.y) == &Background::Goal {
                self.empty_goals += 1;
            }
            if self.background(new.x, new.y) == &Background::Goal {
                self.empty_goals -= 1;
            }
        }

        *self.foreground_mut(new.x, new.y) = *self.foreground_mut(from.x, from.y);
        *self.foreground_mut(from.x, from.y) = Foreground::None;

        (new, from.neighbour(direction.reverse()))
    }

    /// Undo the most recent move.
    pub fn undo(&mut self) {
        if self.moves_recorded == 0 {
            warn!("Nothing to undo!");
            return;
        } else {
            self.moves_recorded -= 1;
        }

        let direction = self.moves[self.moves_recorded].direction;
        let pos = self.worker_position;
        let (worker_pos, crate_pos) = self.move_object(pos, direction, true);
        self.worker_position = worker_pos;

        if self.moves[self.moves_recorded].moves_crate {
            let _ = self.move_object(crate_pos, direction, true);
        }
    }

    /// If a move has been undone previously, redo it.
    pub fn redo(&mut self) {
        if self.moves.len() > self.moves_recorded {
            let dir = self.moves[self.moves_recorded].direction;
            let _ = self.try_move(dir);
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

impl fmt::Display for CurrentLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.level)
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
        let lvl = Level::parse(0,
                               "####\n\
                                #@ #\n\
                                ####\n")
                .unwrap();
        assert_eq!(lvl.worker_position.x, 1);
        assert_eq!(lvl.worker_position.y, 1);
        let mut lvl = CurrentLevel::new(lvl);

        assert!(lvl.is_empty(Position::new(2, 1)));
        assert!(!lvl.is_empty(Position::new(0, 1)));
        for y in 0..3 {
            for x in 0..4 {
                assert!(!lvl.is_crate(Position::new(x, y)));
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
        let lvl = Level::parse(0,
                               "#######\n\
                                #.$@$.#\n\
                                #######\n")
                .unwrap();
        let mut lvl = CurrentLevel::new(lvl);
        assert_eq!(lvl.worker_position.x, 3);
        assert_eq!(lvl.worker_position.y, 1);
        assert!(lvl.try_move(Right).is_ok());
        assert!(lvl.try_move(Left).is_ok());
        assert!(lvl.try_move(Up).is_err());
        assert!(lvl.try_move(Down).is_err());
    }
}
