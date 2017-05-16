use std::convert::TryFrom;
use std::fmt;
use std::collections::VecDeque;

use cell::*;
use direction::*;
use move_::*;
use position::*;
use util::*;

#[derive(Debug, Clone)]
pub struct Level {
    pub rank: usize,
    width: usize,
    height: usize,

    /// `width * height` cells’ backgrounds in row-major order
    pub background: Vec<Background>,

    /// `width * height` cell array of worker and crates in row-major order
    pub foreground: Vec<Foreground>,

    empty_goals: usize,
    pub worker_position: Position,
}


impl Level {
    /// Parse the ASCII representation of a level.
    pub fn parse(num: usize, string: &str) -> Result<Level, SokobanError> {
        let lines: Vec<_> = string.split('\n').filter(|x| !x.is_empty()).collect();
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
               rank: num + 1, // The first level is level 1
               width,
               height,

               background,
               foreground,

               empty_goals,
               worker_position,
           })
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }
}


/// When playing the game, more than just the data stored in a `Level` is needed. For example, we
/// need to record the player’s moves, so we can undo und redo them.
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

    pub fn height(&self) -> usize {
        self.level.height
    }

    pub fn width(&self) -> usize {
        self.level.width
    }

    fn index(&self, pos: Position) -> usize {
        pos.x as usize + pos.y as usize * self.width()
    }

    pub fn background(&self, pos: Position) -> &Background {
        &self.level.background[self.index(pos)]
    }

    pub fn foreground(&self, pos: Position) -> &Foreground {
        &self.level.foreground[self.index(pos)]
    }

    fn foreground_mut(&mut self, pos: Position) -> &mut Foreground {
        let index = self.index(pos);
        &mut self.level.foreground[index]
    }

    /// Try to find a shortest path from the workers current position to `to` and execute it if one
    /// exists.
    pub fn find_path(&mut self, to: Position) {
        let width = self.width();
        let height = self.height();

        if self.worker_position == to {
            return;
        }

        let mut distances = vec![::std::usize::MAX; width * height];
        distances[self.index(to)] = 0;

        let mut found_path = false;
        let mut queue = VecDeque::with_capacity(500);
        queue.push_back(to);

        while let Some(pos) = queue.pop_front() {
            // Have wo found a path?
            if pos == self.worker_position {
                found_path = true;
                break;
            }

            // Is there a neighbour of pos to which we do not currently know the shortest path?
            for neighbour in self.empty_neighbours(pos) {
                let new_dist = distances[self.index(pos)] + 1;

                if distances[self.index(neighbour)] > new_dist {
                    distances[self.index(neighbour)] = new_dist;
                    if neighbour == self.worker_position {
                        // If we get to the source, by the construction of the algorithm, we have
                        // found a shortest path, so we may as well stop the search.
                        queue.truncate(0);
                        found_path = true;
                        break;
                    } else {
                        queue.push_back(neighbour);
                    }
                }
            }
        }

        if found_path {
            // Move worker along the path
            loop {
                for neighbour in self.empty_neighbours(self.worker_position) {
                    if distances[self.index(neighbour)] <
                       distances[self.index(self.worker_position)] {
                        let dir = direction(self.worker_position, neighbour);
                        let _ = self.try_move(dir.unwrap());
                    }
                }
                if self.worker_position == to {
                    break;
                }
            }
        }
    }

    /// A vector of all neighbours of the cell with the given position that contain neither a wall
    /// nor a crate.
    fn empty_neighbours(&self, position: Position) -> Vec<Position> {
        DIRECTIONS
            .iter()
            .map(|&dir| position.neighbour(dir))
            .filter(|&neighbour| self.is_empty(neighbour) || self.is_worker(neighbour))
            .collect()
    }

    /// Move the worker towards `to`. If may_push_crate is set, `to` must be in the same row or
    /// column as the worker. In that case, the worker moves to `to`
    pub fn move_to(&mut self, to: Position, may_push_crate: bool) {
        match direction(self.worker_position, to) {
            Ok(dir) => {
                let (dx, dy) = to - self.worker_position;
                if !may_push_crate && dx.abs() + dy.abs() > 1 {
                    self.find_path(to);
                } else {
                    // Note that this takes care of both movements of just one step and all cases
                    // in which crates may be pushed.
                    while self.move_helper(dir, may_push_crate).is_ok() &&
                          self.worker_position != to {}
                }
            }
            Err(None) => {}// Nothing to do
            Err(_) if !may_push_crate => self.find_path(to),
            Err(_) => error!("Can only move along a row or column when pushing crates"),
        }
    }

    /// Try to move in the given direction. Return an error if that is not possile.
    pub fn try_move(&mut self, direction: Direction) -> Result<(), ()> {
        self.move_helper(direction, true)
    }

    /// Move as far as possible in the given direction (without pushing crates if `may_push_crate`
    /// is `false`).
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
        if pos.x < 0 || pos.y < 0 || pos.x as usize >= self.width() ||
           pos.y as usize >= self.height() {
            return false;
        }

        // Check the cell itself
        self.foreground(pos) == &Foreground::Crate
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_empty(&self, pos: Position) -> bool {
        use self::Background::*;
        use self::Foreground::*;
        let (x, y) = (pos.x as isize, pos.y as isize);

        // Check bounds
        if pos.x < 0 || pos.y < 0 || x as usize >= self.width() || y as usize >= self.height() {
            return false;
        }

        // Check the cell itself
        match (*self.background(pos), *self.foreground(pos)) {
            (Floor, None) | (Goal, None) => true,
            _ => false,
        }
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_worker(&self, pos: Position) -> bool {
        *self.foreground(pos) == Foreground::Worker
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
        // FIXME having these two return values does not seem like a great solution

        // Make sure empty_goals is updated as needed.
        if self.foreground_mut(from) == &Foreground::Crate {
            if self.background(from) == &Background::Goal {
                self.empty_goals += 1;
            }
            if self.background(new) == &Background::Goal {
                self.empty_goals -= 1;
            }
        }

        *self.foreground_mut(new) = *self.foreground_mut(from);
        *self.foreground_mut(from) = Foreground::None;

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
            self.try_move(dir).unwrap();
        }
    }

    /// Check whether the given level is completed, i.e. every goal has a crate on it, and every
    /// crate is on a goal.
    pub fn is_finished(&self) -> bool {
        self.empty_goals == 0
    }

    pub fn moves_to_string(&self) -> String {
        self.moves.iter().map(|mv| mv.to_char()).collect()
    }
}


impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let width = self.width();
        for i in 0..self.height() {
            if i != 0 {
                write!(f, "\n")?;
            }
            for j in 0..width {
                let index = i * width + j;
                let foreground = self.foreground[index];
                let background = self.background[index];
                let cell = Cell {
                    foreground,
                    background,
                };
                write!(f, "{}", cell.to_char())?;
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
