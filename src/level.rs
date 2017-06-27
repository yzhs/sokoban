use std::convert::TryFrom;
use std::fmt;
use std::collections::{VecDeque, HashMap};

use cell::*;
use command::{Response, Obstacle, WithCrate};
use direction::*;
use move_::Move;
use position::*;
use util::*;


#[derive(Debug, Clone)]
pub struct Level {
    pub rank: usize,
    columns: usize,
    rows: usize,

    /// `columns * rows` cells’ backgrounds in row-major order
    pub background: Vec<Background>,

    /// Positions of all crates
    pub crates: HashMap<Position, usize>,

    /// The number of goals that have to be filled to solve the level
    empty_goals: usize,

    /// Where the worker is at the moment
    pub worker_position: Position,

    /// The sequence of moves performed so far. Everything after the first number_of_moves moves is
    /// used to redo moves, i.e. undoing a previous undo operation.
    pub moves: Vec<Move>,

    /// This describes how many moves have to be performed to arrive at the current state.
    number_of_moves: usize,
}


/// Parse level and some basic utility functions. None of these change an existing `Level`.
impl Level {
    /// Parse the ASCII representation of a level.
    pub fn parse(num: usize, string: &str) -> Result<Level, SokobanError> {
        let lines: Vec<_> = string.lines()
            // Skip empty lines and comments
            .filter(|x| !x.is_empty() && !x.trim().starts_with(';'))
            .collect();
        let rows = lines.len();
        let columns = lines.iter().map(|x| x.len()).max().unwrap();

        let mut found_worker = false;
        let mut worker_position = Position { x: 0, y: 0 };
        let mut empty_goals = 0;
        let mut background = vec![Background::Empty; columns * rows];
        let mut crates = Vec::with_capacity(20);

        let mut goals_minus_crates = 0_i32;

        let mut found_level_description = false;
        for (i, line) in lines.iter().enumerate() {
            let mut inside = false;
            for (j, chr) in line.chars().enumerate() {
                let cell = Cell::try_from(chr)
                    .expect(format!("Invalid character '{}' in line {}, column {}.", chr, i, j)
                                .as_ref());
                let index = i * columns + j;
                background[index] = cell.background;
                found_level_description = true;

                // Count goals still to be filled and make sure that there are exactly as many
                // goals as there are crates.
                if cell.background == Background::Goal && cell.foreground != Foreground::Crate {
                    empty_goals += 1;
                    goals_minus_crates += 1;
                } else if cell.background != Background::Goal &&
                          cell.foreground == Foreground::Crate {
                    goals_minus_crates -= 1;
                }
                if cell.foreground == Foreground::Crate {
                    crates.push(Position::new(j, i));
                }

                // Try to figure out whether a given cell is inside the walls.
                if !inside && cell.background.is_wall() {
                    inside = true;
                }

                if inside && cell.background == Background::Empty && index >= columns &&
                   background[index - columns] != Background::Empty {
                    background[index] = Background::Floor;
                }

                // Find the initial worker position.
                if cell.foreground == Foreground::Worker {
                    if found_worker {
                        return Err(SokobanError::TwoWorkers(num + 1));
                    }
                    worker_position = Position::new(j, i);
                    found_worker = true;
                }
            }
        }
        if !found_level_description {
            return Err(SokobanError::NoLevel(num + 1));
        } else if !found_worker {
            return Err(SokobanError::NoWorker(num + 1));
        } else if goals_minus_crates != 0 {
            return Err(SokobanError::CratesGoalsMismatch(num + 1, goals_minus_crates));
        }

        // Fix the mistakes of the above heuristic for detecting which cells are on the inside.
        let mut queue = VecDeque::new();
        let mut visited = vec![false; background.len()];
        let mut inside = vec![false; background.len()];

        visited[worker_position.to_index(columns)] = true;
        inside[worker_position.to_index(columns)] = true;
        queue.push_back(worker_position);

        for crate_pos in &crates {
            visited[crate_pos.to_index(columns)] = true;
            queue.push_back(*crate_pos);
        }

        for (i, &bg) in background.iter().enumerate() {
            match bg {
                Background::Wall => visited[i] = true,
                Background::Goal if !visited[i] => {
                    inside[i] = true;
                    visited[i] = true;
                    queue.push_back(Position::from_index(i, columns));
                }
                _ => (),
            }
        }

        // Flood fill from all positions added above
        while let Some(pos) = queue.pop_front() {
            use Direction::*;
            let i = pos.to_index(columns);
            if let Background::Wall = background[i] {
                continue;
            } else {
                inside[i] = true;
            }
            for n in [Up, Down, Left, Right].iter().map(|&x| pos.neighbour(x)) {
                // The outermost rows and columns may only contain empty space and walls, so
                // n has to bee within bounds.
                let j = n.to_index(columns);
                if !visited[j] {
                    visited[j] = true;
                    queue.push_back(n);
                }
            }
        }

        for (i, bg) in background.iter_mut().enumerate() {
            if !inside[i] && *bg == Background::Floor {
                *bg = Background::Empty;
            }
        }

        Ok(Level {
               rank: num + 1, // The first level is level 1
               columns,
               rows,

               background,
               crates: crates
                   .into_iter()
                   .enumerate()
                   .map(|(i, x)| (x, i))
                   .collect(),

               empty_goals,
               worker_position,

               moves: vec![],
               number_of_moves: 0,
           })
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    fn index(&self, pos: Position) -> usize {
        pos.x as usize + pos.y as usize * self.columns()
    }

    pub fn position(&self, i: usize) -> Position {
        Position::new(i % self.columns, i / self.columns)
    }

    pub fn background(&self, pos: Position) -> &Background {
        &self.background[self.index(pos)]
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

    fn in_bounds(&self, pos: Position) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.columns() as isize && pos.y < self.rows() as isize
    }

    /// Is there a crate at the given position?
    fn is_crate(&self, pos: Position) -> bool {

        // Check the cell itself
        self.crates.get(&pos).is_some()
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_empty(&self, pos: Position) -> bool {
        self.is_interior(pos) && !self.is_crate(pos)
    }

    pub fn is_outside(&self, pos: Position) -> bool {
        !self.in_bounds(pos) || *self.background(pos) == Background::Empty
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_worker(&self, pos: Position) -> bool {
        pos == self.worker_position
    }

    /// The cell at the given position is neither empty, nor does it contain a wall.
    pub fn is_interior(&self, pos: Position) -> bool {
        use self::Background::*;

        if !self.in_bounds(pos) {
            return false;
        }

        match *self.background(pos) {
            Floor | Goal => true,
            _ => false,
        }
    }


    /// Check whether the given level is completed, i.e. every goal has a crate on it, and every
    /// crate is on a goal.
    pub fn is_finished(&self) -> bool {
        self.empty_goals == 0
    }

    /// How moves were performed to reach the current state?
    pub fn number_of_moves(&self) -> usize {
        self.number_of_moves
    }

    /// How many times have crates been moved to reach the current state?
    pub fn number_of_pushes(&self) -> usize {
        self.moves[0..self.number_of_moves]
            .iter()
            .filter(|x| x.moves_crate)
            .count()
    }

    /// Which direction is the worker currently facing?
    pub fn worker_direction(&self) -> Direction {
        if self.number_of_moves == 0 {
            Direction::Left
        } else {
            self.moves[self.number_of_moves - 1].direction
        }
    }

    /// Create a string representation of the moves made to reach the current state.
    pub fn moves_to_string(&self) -> String {
        self.moves
            .iter()
            .take(self.number_of_moves)
            .map(|mv| mv.to_char())
            .collect()
    }
}


/// Movement, i.e. everything that *does* change the `self`.
impl Level {
    /// Move whatever object is in the foreground at the given position in the given direction if
    /// undo is false, and in the opposite direction otherwise. Return the new position of that
    /// object, as well as the position of the object behind the original position. This is needed
    /// to move crates backwards when undoing a push.
    fn move_object(&mut self, from: Position, direction: Direction, undo: bool) -> Position {
        let direction = if undo { direction.reverse() } else { direction };
        let new = from.neighbour(direction);

        // Make sure empty_goals is updated as needed.
        if self.is_crate(from) {
            if self.background(from) == &Background::Goal {
                self.empty_goals += 1;
            }
            if self.background(new) == &Background::Goal {
                self.empty_goals -= 1;
            }
            let id = self.crates.remove(&from).unwrap();
            self.crates.insert(new, id);
        } else {
            self.worker_position = new;
        }

        new
    }

    /// Move one step in the given direction if that cell is empty or `may_push_crate` is true and
    /// the next cell contains a crate which can be pushed in the given direction.
    fn move_helper(&mut self,
                   direction: Direction,
                   may_push_crate: bool)
                   -> Result<Vec<Response>, Response> {
        let mut result = vec![];
        let next = self.worker_position.neighbour(direction);
        let next_but_one = next.neighbour(direction);

        let moves_crate = if self.is_empty(next) {
            // Move to empty cell
            false
        } else if self.is_crate(next) && self.is_empty(next_but_one) &&
                  may_push_crate {
            // Push crate into empty next cell
            self.move_object(next, direction, false);
            result.push(Response::MoveCrateTo(self.crates[&next_but_one], next_but_one));
            true
        } else {
            let b = may_push_crate && self.is_crate(next);
            let obj = if b && self.is_crate(next_but_one) {
                Obstacle::Crate
            } else {
                Obstacle::Wall
            };
            return Err(Response::CannotMove(WithCrate(b), obj));
        };

        // Move worker to new position
        let pos = self.worker_position;
        let worker_pos = self.move_object(pos, direction, false);
        self.worker_position = worker_pos;
        result.push(Response::MoveWorkerTo(worker_pos, direction));

        // Bookkeeping for undo and printing a solution
        let current_move = Move {
            direction,
            moves_crate,
        };
        let n = self.number_of_moves;
        self.number_of_moves += 1;

        if n != self.moves.len() && self.moves[n] == current_move {
            // In this case, we are just redoing a move previously undone
        } else {
            if n != self.moves.len() {
                // Discard redo buffer as we are in a different state than before
                self.moves.truncate(n);
            }
            self.moves.push(current_move);
        }

        Ok(result)
    }


    /// Move the worker towards `to`. If may_push_crate is set, `to` must be in the same row or
    /// column as the worker. In that case, the worker moves to `to`
    pub fn move_to(&mut self, to: Position, may_push_crate: bool) -> Vec<Response> {
        match direction(self.worker_position, to) {
            Ok(dir) => {
                let (dx, dy) = to - self.worker_position;
                if !may_push_crate && dx.abs() + dy.abs() > 1 {
                    self.find_path(to).unwrap_or_default()
                } else {
                    let mut result = vec![];
                    // Note that this takes care of both movements of just one step and all cases
                    // in which crates may be pushed.
                    while let Ok(resp) = self.move_helper(dir, may_push_crate) {
                        result.extend(resp);
                        if self.worker_position == to || may_push_crate && self.is_finished() {
                            break;
                        }
                    }
                    result
                }
            }
            Err(None) => vec![],
            Err(_) if !may_push_crate => self.find_path(to).unwrap_or_default(),
            Err(_) => vec![Response::NoPathfindingWhilePushing],
        }
    }

    /// Try to move in the given direction. Return an error if that is not possile.
    pub fn try_move(&mut self, direction: Direction) -> Vec<Response> {
        match self.move_helper(direction, true) {
            Ok(resp) => resp,
            Err(e) => vec![e],
        }
    }

    /// Try to find a shortest path from the workers current position to `to` and execute it if one
    /// exists.
    pub fn find_path(&mut self, to: Position) -> Result<Vec<Response>, ()> {
        let mut result = vec![];
        let columns = self.columns();
        let rows = self.rows();

        if self.worker_position == to || !self.is_empty(to) {
            return Ok(result);
        }

        let mut distances = vec![::std::usize::MAX; columns * rows];
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
                        result.extend(self.try_move(dir.unwrap()))
                    }
                }
                if self.worker_position == to {
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Move as far as possible in the given direction (without pushing crates if `may_push_crate`
    /// is `false`).
    pub fn move_until(&mut self,
                      direction: Direction,
                      may_push_crate: bool)
                      -> Result<Vec<Response>, ()> {
        let mut result = vec![];
        while let Ok(resp) = self.move_helper(direction, may_push_crate) {
            result.extend(resp);
            if may_push_crate && self.is_finished() {
                break;
            }
        }
        // TODO handle errors
        Ok(result)
    }

    /// Undo the most recent move.
    pub fn undo(&mut self) -> Vec<Response> {
        if self.number_of_moves == 0 {
            warn!("Nothing to undo!");
            return vec![Response::NothingToRedo];
        } else {
            self.number_of_moves -= 1;
        }
        let mut result = vec![];

        let direction = self.moves[self.number_of_moves].direction;
        let pos = self.worker_position;
        let worker_pos = self.move_object(pos, direction, true);
        let crate_pos = pos.neighbour(direction);

        if self.moves[self.number_of_moves].moves_crate {
            let new = self.move_object(crate_pos, direction, true);
            result.push(Response::MoveCrateTo(self.crates[&new], new));
        }
        result.push(Response::MoveWorkerTo(worker_pos, self.worker_direction()));

        result
    }

    /// If a move has been undone previously, redo it.
    pub fn redo(&mut self) -> Vec<Response> {
        if self.moves.len() > self.number_of_moves {
            let dir = self.moves[self.number_of_moves].direction;
            self.try_move(dir)
        } else {
            vec![Response::NothingToRedo]
        }
    }

    pub fn execute_moves(&mut self, number_of_moves: usize, moves: &str) {
        let moves = ::move_::parse(moves).unwrap();
        // TODO Error handling
        for (i, move_) in moves.iter().enumerate() {
            // Some moves might have been undone, so we do not redo them just now.
            if i >= number_of_moves {
                self.moves = moves.to_owned();
                break;
            }
            let res = self.try_move(move_.direction);
            if res.len() == 1 && res[0].is_error() {
                error!("{:?}", res[0]);
            }
        }
    }

    pub fn all_moves_to_string(&self) -> String {
        let mut result = String::with_capacity(self.moves.len());
        for mv in &self.moves {
            result.push(mv.to_char());
        }
        result
    }
}


impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let columns = self.columns();
        for i in 0..self.rows() {
            if i != 0 {
                write!(f, "\n")?;
            }
            for j in 0..columns {
                let background = self.background[j + i * self.columns];
                let pos = Position::new(j, i);
                let foreground = if self.worker_position == pos {
                    Foreground::Worker
                } else if self.is_crate(pos) {
                    Foreground::Crate
                } else {
                    Foreground::None
                };
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

    fn contains_error(responses: &[Response]) -> bool {
        responses.iter().any(|x| x.is_error())
    }

    #[test]
    fn test_trivial_move_1() {
        use self::Direction::*;
        let mut lvl = Level::parse(0,
                                   "####\n\
                                    #@ #\n\
                                    ####\n")
                .unwrap();
        assert_eq!(lvl.worker_position.x, 1);
        assert_eq!(lvl.worker_position.y, 1);

        assert!(lvl.is_empty(Position::new(2, 1)));
        assert!(!lvl.is_empty(Position::new(0, 1)));
        for y in 0..3 {
            for x in 0..4 {
                assert!(!lvl.is_crate(Position::new(x, y)));
            }
        }

        assert!(!contains_error(&lvl.try_move(Right)));
        assert!(!contains_error(&lvl.try_move(Left)));
        assert!(contains_error(&lvl.try_move(Left)));
        assert!(contains_error(&lvl.try_move(Up)));
        assert!(contains_error(&lvl.try_move(Down)));
    }

    #[test]
    fn test_trivial_move_2() {
        use self::Direction::*;
        let mut lvl = Level::parse(0,
                                   "#######\n\
                                    #.$@$.#\n\
                                    #######\n")
                .unwrap();
        assert_eq!(lvl.worker_position.x, 3);
        assert_eq!(lvl.worker_position.y, 1);
        assert!(!contains_error(&lvl.try_move(Right)));
        assert!(!contains_error(&lvl.try_move(Left)));
        assert!(contains_error(&lvl.try_move(Up)));
        assert!(contains_error(&lvl.try_move(Down)));
    }
}
