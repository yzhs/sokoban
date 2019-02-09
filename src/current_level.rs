pub mod graph;
pub mod pathfinding;

use std::{collections::HashMap, fmt, sync::mpsc::Sender};

use crate::command::{Obstacle, WithCrate};
use crate::current_level::pathfinding::*;
use crate::direction::*;
use crate::event::Event;
use crate::level::builder::{Foreground, LevelBuilder};
use crate::level::{Background, Level};
use crate::move_::Move;
use crate::position::*;
use crate::util::*;

#[derive(Debug, Clone)]
pub struct CurrentLevel {
    rank: usize,
    columns: usize,
    rows: usize,

    /// `columns * rows` cells’ backgrounds in row-major order
    background: Vec<Background>,

    /// Positions of all crates
    crates: HashMap<Position, usize>,

    /// The number of goals that have to be filled to solve the level
    empty_goals: usize,

    /// Where the worker is at the moment
    worker_position: Position,

    /// The sequence of moves performed so far. Everything after the first number_of_moves moves is
    /// used to redo moves, i.e. undoing a previous undo operation.
    moves: Vec<Move>,

    /// This describes how many moves have to be performed to arrive at the current state.
    number_of_moves: usize,

    listeners: Vec<Sender<Event>>,
}

/// Parse level and some basic utility functions. None of these change an existing `CurrentLevel`. {{{
impl CurrentLevel {
    /// Parse the ASCII representation of a level.
    pub fn parse(num: usize, string: &str) -> Result<CurrentLevel, SokobanError> {
        let builder = LevelBuilder::new(num + 1, string)?;
        Ok(builder.build().into())
    }

    pub fn rank(&self) -> usize {
        self.rank
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn worker_position(&self) -> Position {
        self.worker_position
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

    /// Get an ordered list of the crates’ positions where the id of a crate is its index in the
    /// list.
    pub fn crate_positions(&self) -> Vec<Position> {
        let mut crates: Vec<_> = self.crates.iter().collect();
        crates.sort_by_key(|&(_pos, id)| id);
        crates.into_iter().map(|(&pos, _id)| pos).collect()
    }

    pub fn background_cells(&self) -> &[Background] {
        self.background.as_ref()
    }
}
// }}}

/// Emit the appropriate events {{{
impl CurrentLevel {
    pub fn subscribe(&mut self, sender: Sender<Event>) {
        self.listeners.push(sender);
    }

    fn notify(&self, event: &Event) {
        for sender in &self.listeners {
            sender.send(event.clone()).unwrap();
        }
    }

    fn move_worker(&mut self, direction: Direction) {
        let to = self.worker_position.neighbour(direction);
        self.move_worker_to(to, direction);
    }

    fn move_worker_back(&mut self, direction: Direction) {
        let to = self.worker_position.neighbour(direction.reverse());
        let from = self.worker_position;
        self.worker_position = to;
        self.on_worker_move(from, to, direction);
    }

    fn move_worker_to(&mut self, to: Position, direction: Direction) {
        let from = self.worker_position;
        self.worker_position = to;
        self.on_worker_move(from, to, direction);
    }

    fn on_worker_move(&self, from: Position, to: Position, direction: Direction) {
        let event = Event::MoveWorker {
            from,
            to,
            direction,
        };
        self.notify(&event);
    }

    fn move_crate(&mut self, from: Position, direction: Direction) {
        self.move_crate_to(from, from.neighbour(direction));
    }

    // NOTE We need `from` so we can find out the crate's id. That way, the user interface knows
    // which crate to animate. Alternatively, the crate's id could be passed in.
    fn move_crate_to(&mut self, from: Position, to: Position) {
        let id = self.crates.remove(&from).unwrap();
        self.crates.insert(to, id);

        if self.background(from) == &Background::Goal {
            self.empty_goals += 1;
        }
        if self.background(to) == &Background::Goal {
            self.empty_goals -= 1;
        }

        self.on_crate_move(id, from, to);
    }

    fn on_crate_move(&self, id: usize, from: Position, to: Position) {
        let event = Event::MoveCrate { id, from, to };
        self.notify(&event);
    }
}
// }}}

/// Movement, i.e. everything that *does* change the `self`.
impl CurrentLevel {
    /// Move one step in the given direction if that cell is empty or `may_push_crate` is true and
    /// the next cell contains a crate which can be pushed in the given direction.
    fn move_helper(&mut self, direction: Direction, may_push_crate: bool) -> Result<(), Event> {
        let next = self.worker_position.neighbour(direction);
        let next_but_one = next.neighbour(direction);

        let moves_crate = if self.is_empty(next) {
            false
        } else if self.is_crate(next) && self.is_empty(next_but_one) && may_push_crate {
            self.move_crate(next, direction);
            true
        } else {
            let b = may_push_crate && self.is_crate(next);
            let obj = if b && self.is_crate(next_but_one) {
                Obstacle::Crate
            } else {
                Obstacle::Wall
            };
            // TODO make sure the result is used when appropriate
            return Err(Event::CannotMove(WithCrate(b), obj));
        };

        self.move_worker(direction);

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

        Ok(())
    }

    /// Move the worker towards `to`. If may_push_crate is set, `to` must be in the same row or
    /// column as the worker. In that case, the worker moves to `to`
    pub fn move_to(&mut self, to: Position, may_push_crate: bool) -> Option<()> {
        let dir = direction(self.worker_position, to);

        if !may_push_crate {
            let (dx, dy) = to - self.worker_position;
            if dx.abs() + dy.abs() > 1 {
                let path = self.find_path(to)?;
                self.follow_path(path);
                return Some(());
            }
        }

        match dir {
            DirectionResult::Neighbour { direction } => {
                // Note that this takes care of both movements of just one step and all cases
                // in which crates may be pushed.
                while self.move_helper(direction, may_push_crate).is_ok() {
                    if self.worker_position == to || may_push_crate && self.is_finished() {
                        break;
                    }
                }
            }
            DirectionResult::SamePosition => {}
            DirectionResult::Other => self.notify(&Event::NoPathfindingWhilePushing),
        }

        Some(())
    }

    /// Try to move in the given direction. Return an error if that is not possible.
    pub fn try_move(&mut self, direction: Direction) -> Result<(), Event> {
        self.move_helper(direction, true)
    }

    /// Move the crate located at `from` to `to` if that is possible.
    pub fn move_crate_to_target(&mut self, from: Position, to: Position) -> Option<()> {
        let path = self.find_path_with_crate(from, to)?;

        info!("Found a path from {:?} to {:?}", from, to);
        self.push_crate_along_path(path)
    }

    /// Move as far as possible in the given direction (without pushing crates if `may_push_crate`
    /// is `false`).
    pub fn move_as_far_as_possible(&mut self, direction: Direction, may_push_crate: bool) {
        while self.move_helper(direction, may_push_crate).is_ok()
            && !(may_push_crate && self.is_finished())
        {}
    }

    /// Undo the most recent move.
    pub fn undo(&mut self) -> bool {
        if self.number_of_moves == 0 {
            self.notify(&Event::NothingToUndo);
            return false;
        } else {
            self.number_of_moves -= 1;
        }

        let direction = self.moves[self.number_of_moves].direction;
        let crate_pos = self.worker_position.neighbour(direction);
        self.move_worker_back(direction);

        if self.moves[self.number_of_moves].moves_crate {
            self.move_crate(crate_pos, direction.reverse());
        }

        true
    }

    /// If a move has been undone previously, redo it.
    pub fn redo(&mut self) -> bool {
        if self.moves.len() > self.number_of_moves {
            let dir = self.moves[self.number_of_moves].direction;
            let is_ok = self.try_move(dir).is_ok();
            assert!(is_ok);
            true
        } else {
            self.notify(&Event::NothingToRedo);
            false
        }
    }

    /// Given a number of simple moves, i.e. up, down, left, right, as a string, execute the first
    /// `number_of_moves` of them. If there are more moves than that, they can be executed using
    /// redo.
    pub fn execute_moves(&mut self, number_of_moves: usize, moves: &str) -> Result<(), Event> {
        let moves = crate::move_::parse(moves).unwrap();
        // TODO Error handling
        for (i, move_) in moves.iter().enumerate() {
            // Some moves might have been undone, so we do not redo them just now.
            if i >= number_of_moves {
                self.moves = moves.to_owned();
                break;
            }
            self.try_move(move_.direction)?;
        }

        Ok(())
    }

    /// Convert moves to string, including moves that have been undone.
    pub fn all_moves_to_string(&self) -> String {
        let mut result = String::with_capacity(self.moves.len());
        for mv in &self.moves {
            result.push(mv.to_char());
        }
        result
    }
}

fn cell_to_char(background: Background, foreground: Foreground) -> char {
    match (background, foreground) {
        (Background::Wall, Foreground::None) => '#',
        (Background::Empty, Foreground::None) | (Background::Floor, Foreground::None) => ' ',
        (Background::Floor, Foreground::Crate) => '$',
        (Background::Floor, Foreground::Worker) => '@',
        (Background::Goal, Foreground::None) => '.',
        (Background::Goal, Foreground::Crate) => '*',
        (Background::Goal, Foreground::Worker) => '+',
        _ => panic!(
            "Invalid combination: {:?} on top of {:?}",
            foreground, background
        ),
    }
}

impl fmt::Display for CurrentLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let columns = self.columns();
        for i in 0..self.rows() {
            if i != 0 {
                writeln!(f)?;
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
                let cell = cell_to_char(background, foreground);
                write!(f, "{}", cell)?;
            }
        }
        Ok(())
    }
}

impl From<&Level> for CurrentLevel {
    fn from(level: &Level) -> Self {
        Self {
            rank: level.rank,
            columns: level.columns,
            rows: level.rows,
            background: level.background.clone(),
            crates: level.crates.clone(),
            worker_position: level.worker_position,
            empty_goals: level.empty_goals,
            moves: level.moves.clone(),
            number_of_moves: level.number_of_moves,
            listeners: level.listeners.clone(),
        }
    }
}

impl From<Level> for CurrentLevel {
    fn from(level: Level) -> Self {
        (&level).into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_trivial_move_1() {
        use self::Direction::*;

        let mut lvl = CurrentLevel::parse(
            0,
            "####\n\
             #@ #\n\
             ####\n",
        )
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

        assert!(!&lvl.try_move(Right).is_err());
        assert!(!&lvl.try_move(Left).is_err());
        assert!(&lvl.try_move(Left).is_err());
        assert!(&lvl.try_move(Up).is_err());
        assert!(&lvl.try_move(Down).is_err());
    }

    #[test]
    fn test_trivial_move_2() {
        use self::Direction::*;
        let mut lvl = CurrentLevel::parse(
            0,
            "#######\n\
             #.$@$.#\n\
             #######\n",
        )
        .unwrap();
        assert_eq!(lvl.worker_position.x, 3);
        assert_eq!(lvl.worker_position.y, 1);
        assert_eq!(lvl.worker_direction(), Left);
        assert!(!&lvl.try_move(Right).is_err());
        assert!(!&lvl.try_move(Left).is_err());
        assert!(!&lvl.try_move(Left).is_err());
        assert!(&lvl.try_move(Up).is_err());
        assert!(&lvl.try_move(Down).is_err());
        assert!(lvl.is_finished());
        assert!(lvl.undo());
        assert!(!lvl.is_finished());
        assert!(!&lvl.try_move(Right).is_err());
        assert_eq!(lvl.worker_direction(), Right);
        assert!(!lvl.redo());
        assert!(!&lvl.try_move(Left).is_err());
        assert!(!&lvl.try_move(Left).is_err());
        assert!(lvl.is_finished());
        assert_eq!(lvl.worker_direction(), Left);
    }
}
