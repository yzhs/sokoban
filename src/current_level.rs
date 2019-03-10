pub mod graph;
pub mod pathfinding;

use std::{collections::HashMap, fmt, sync::mpsc::Sender};

use crate::command::Obstacle;
use crate::direction::*;
use crate::event::Event;
use crate::level::builder::Foreground;
use crate::level::{Background, Level};
use crate::move_::Move;
use crate::position::*;
use crate::undo::Undo;

#[derive(Clone)]
pub struct DynamicEntities {
    /// Positions of all crates
    crates: HashMap<Position, usize>,

    /// The number of goals that have to be filled to solve the level
    empty_goals: usize,

    /// Where the worker is at the moment
    worker_position: Position,
}

impl DynamicEntities {
    pub fn is_empty(&self, position: Position) -> bool {
        !self.is_crate(position)
    }

    /// Is there a crate at the given position?
    fn is_crate(&self, pos: Position) -> bool {
        self.crates.get(&pos).is_some()
    }
}

#[derive(Clone)]
pub struct CurrentLevel {
    columns: usize,
    rows: usize,

    /// `columns * rows` cells’ backgrounds in row-major order
    background: Vec<Background>,

    dynamic: DynamicEntities,

    undo: Undo<Move>,

    listeners: Vec<Sender<Event>>,
}

/// Parse level and some basic utility functions. None of these change an existing `CurrentLevel`. {{{
impl CurrentLevel {
    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn worker_position(&self) -> Position {
        self.dynamic.worker_position
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
        self.dynamic.is_crate(pos)
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_empty(&self, pos: Position) -> bool {
        self.is_interior(pos) && self.dynamic.is_empty(pos)
    }

    pub fn is_outside(&self, pos: Position) -> bool {
        !self.in_bounds(pos) || *self.background(pos) == Background::Empty
    }

    /// Is the cell with the given coordinates empty, i.e. could a crate be moved into it?
    fn is_worker(&self, pos: Position) -> bool {
        pos == self.dynamic.worker_position
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
        self.dynamic.empty_goals == 0
    }

    /// How moves were performed to reach the current state?
    pub fn number_of_moves(&self) -> usize {
        self.undo.number_of_actions()
    }

    /// How many times have crates been moved to reach the current state?
    pub fn number_of_pushes(&self) -> usize {
        self.undo.count_matches(|x| x.moves_crate)
    }

    /// Which direction is the worker currently facing?
    pub fn worker_direction(&self) -> Direction {
        if self.undo.is_empty() {
            Direction::Left
        } else {
            self.undo.last().direction
        }
    }

    /// Create a string representation of the moves made to reach the current state.
    pub fn moves_to_string(&self) -> String {
        self.undo.to_string(Move::to_char)
    }

    /// Get an ordered list of the crates’ positions where the id of a crate is its index in the
    /// list.
    pub fn crate_positions(&self) -> Vec<Position> {
        let mut crates: Vec<_> = self.dynamic.crates.iter().collect();
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

    fn move_worker_from_to(&mut self, from_to: FromTo) -> Event {
        let FromTo { from, to } = from_to;
        if let DirectionResult::Neighbour { direction } = direction(from, to) {
            self.move_worker_to(to, direction)
        } else {
            panic!("invalid FromTo: {:?}", from_to)
        }
    }

    fn move_worker(&mut self, direction: Direction) -> Event {
        let to = self.dynamic.worker_position.neighbour(direction);
        self.move_worker_to(to, direction)
    }

    fn move_worker_back(&mut self, direction: Direction) -> Event {
        let to = self.dynamic.worker_position.neighbour(direction.reverse());
        let from = self.dynamic.worker_position;
        self.dynamic.worker_position = to;

        Event::MoveWorker {
            from,
            to,
            direction,
        }
    }

    fn move_worker_to(&mut self, to: Position, direction: Direction) -> Event {
        let from = self.dynamic.worker_position;
        self.dynamic.worker_position = to;

        Event::MoveWorker {
            from,
            to,
            direction,
        }
    }

    fn move_crate(&mut self, from: Position, direction: Direction) -> Event {
        self.move_crate_to(from, from.neighbour(direction))
    }

    // NOTE We need `from` so we can find out the crate's id. That way, the user interface knows
    // which crate to animate. Alternatively, the crate's id could be passed in.
    fn move_crate_to(&mut self, from: Position, to: Position) -> Event {
        let id = self.dynamic.crates.remove(&from).unwrap_or_else(|| {
            panic!(
                "Moving crate from {:?} to {:?}. Crates: {:?}",
                from, to, self.dynamic.crates
            )
        });
        self.dynamic.crates.insert(to, id);

        if self.background(from) == &Background::Goal {
            self.dynamic.empty_goals += 1;
        }
        if self.background(to) == &Background::Goal {
            self.dynamic.empty_goals -= 1;
        }

        Event::MoveCrate { id, from, to }
    }
}
// }}}

#[derive(Debug)]
pub struct FromTo {
    from: Position,
    to: Position,
}

pub enum BlockedEntity {
    Worker,
    Crate,
}

pub struct VerifiedMove {
    worker_move: FromTo,
    crate_move: Option<FromTo>,
}

pub struct FailedMove {
    pub obstacle_at: Position,
    pub obstacle_type: Obstacle,
    pub thing_blocked: BlockedEntity,
}

/// Public movement functions.
impl CurrentLevel {
    /// Take one step in the specified direction, pushing a crate if necessary.
    pub fn step(&mut self, direction: Direction) {
        if let Err(event) = self.try_move(direction) {
            self.notify(&event.into());
        }
    }

    /// Walk in the given direction until the first obstacle is reached. Do not push any crates.
    pub fn walk_to_obstacle(
        &mut self,
        direction: Direction,
        dynamic: &mut DynamicEntities,
    ) -> Result<Vec<VerifiedMove>, FailedMove> {
        let mut moves = vec![];

        loop {
            let next_position = dynamic.worker_position.neighbour(direction);

            if !self.is_empty(next_position) {
                break;
            }

            moves.push(VerifiedMove {
                worker_move: FromTo {
                    from: dynamic.worker_position,
                    to: next_position,
                },
                crate_move: None,
            });

            dynamic.worker_position = next_position;
        }

        Ok(moves)
    }
}

/// Movement, i.e. everything that *does* change the `self`.
impl CurrentLevel {
    pub fn perform_moves(&mut self, moves: &[Move]) -> Result<Vec<Event>, FailedMove> {
        let mut events = vec![];

        for r#move in moves {
            events.append(&mut self.perform_move(r#move, true)?);
        }

        Ok(events)
    }

    fn perform_move(&mut self, r#move: &Move, record_move: bool) -> Result<Vec<Event>, FailedMove> {
        // DEBT get rid of record_move!
        let VerifiedMove {
            worker_move,
            crate_move,
        } = self.evaluate_move(r#move)?;

        let mut events = vec![];
        if let Some(FromTo { from, to }) = crate_move {
            events.push(self.move_crate_to(from, to));
        }

        events.push(self.move_worker_from_to(worker_move));

        if record_move {
            self.undo.record(r#move.to_owned());
        }

        Ok(events)
    }

    /// Figure out whether a `Move` can be performed at the current state. If so, return what
    /// changes it causes. Otherwise, return why it cannot be performed.
    fn evaluate_move(&self, r#move: &Move) -> Result<VerifiedMove, FailedMove> {
        let dynamic = &self.dynamic;

        let Move {
            moves_crate,
            direction,
        } = r#move;
        let new_worker_position = dynamic.worker_position.neighbour(*direction);

        let is_crate = self.is_crate(new_worker_position);

        if is_crate && *moves_crate {
            let new_crate_position = new_worker_position.neighbour(*direction);

            if self.is_interior(new_worker_position) && dynamic.is_empty(new_crate_position) {
                Ok(VerifiedMove {
                    worker_move: FromTo {
                        from: dynamic.worker_position,
                        to: new_worker_position,
                    },
                    crate_move: Some(FromTo {
                        from: new_worker_position,
                        to: new_crate_position,
                    }),
                })
            } else {
                let obstacle = match *self.background(new_crate_position) {
                    Background::Wall => Obstacle::Wall,
                    _ => Obstacle::Crate,
                };

                Err(FailedMove {
                    obstacle_at: new_crate_position,
                    obstacle_type: obstacle,
                    thing_blocked: BlockedEntity::Crate,
                })
            }
        } else if self.is_interior(new_worker_position) && dynamic.is_empty(new_worker_position) {
            Ok(VerifiedMove {
                worker_move: FromTo {
                    from: dynamic.worker_position,
                    to: new_worker_position,
                },
                crate_move: None,
            })
        } else {
            let obstacle_type = if is_crate {
                Obstacle::Crate
            } else {
                Obstacle::Wall
            };
            Err(FailedMove {
                obstacle_at: new_worker_position,
                obstacle_type,
                thing_blocked: BlockedEntity::Worker,
            })
        }
    }

    /// Move one step in the given direction if that cell is empty or `may_push_crate` is true and
    /// the next cell contains a crate which can be pushed in the given direction.
    fn move_helper(
        &mut self,
        direction: Direction,
        may_push_crate: bool,
    ) -> Result<(), FailedMove> {
        let target_position = self.dynamic.worker_position.neighbour(direction);
        let is_crate = self.dynamic.crates.contains_key(&target_position);

        let events = self.perform_move(
            &Move {
                direction,
                moves_crate: may_push_crate && is_crate,
            },
            true,
        )?;
        // FIXME properly handle errors

        for event in events {
            self.notify(&event);
        }

        Ok(())
    }

    /// Move the worker towards `to`. If may_push_crate is set, `to` must be in the same row or
    /// column as the worker. In that case, the worker moves to `to`
    pub fn move_to(&mut self, to: Position, may_push_crate: bool) -> Option<()> {
        let dir = direction(self.dynamic.worker_position, to);

        if !may_push_crate {
            let (dx, dy) = to - self.dynamic.worker_position;
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
                    if self.dynamic.worker_position == to || may_push_crate && self.is_finished() {
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
    pub fn try_move(&mut self, direction: Direction) -> Result<(), FailedMove> {
        self.move_helper(direction, true)
    }

    /// Move the crate located at `from` to `to` if that is possible.
    pub fn move_crate_to_target(&mut self, from: Position, to: Position) -> Option<()> {
        let path = self.find_path_with_crate(from, to)?;

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
        match self.undo.undo() {
            None => {
                self.notify(&Event::NothingToUndo);
                false
            }
            Some(&Move {
                direction,
                moves_crate,
            }) => {
                let crate_pos = self.dynamic.worker_position.neighbour(direction);

                let event = self.move_worker_back(direction);
                self.notify(&event);

                if moves_crate {
                    let event = self.move_crate(crate_pos, direction.reverse());
                    self.notify(&event);
                }

                true
            }
        }
    }

    /// If a move has been undone previously, redo it.
    pub fn redo(&mut self) -> bool {
        let r#move = if let Some(r#move) = self.undo.redo() {
            r#move.to_owned()
        } else {
            self.notify(&Event::NothingToRedo);
            return false;
        };

        match self.perform_move(&r#move, false) {
            Ok(events) => {
                for event in events {
                    self.notify(&event);
                }
                true
            }
            Err(err) => {
                self.notify(&err.into());
                false
            }
        }
    }

    /// Given a number of simple moves, i.e. up, down, left, right, as a string, execute the first
    /// `number_of_moves` of them. If there are more moves than that, they can be executed using
    /// redo.
    ///
    /// Used for loading a level.
    pub fn execute_moves(&mut self, number_of_moves: usize, moves: &str) -> Result<(), FailedMove> {
        // DEBT Should be moved somewhere else. load.rs, maybe?
        let moves = crate::move_::parse(moves).unwrap();
        // TODO Error handling
        for (i, move_) in moves.iter().enumerate() {
            // Some moves might have been undone, so we do not redo them just now.
            if i >= number_of_moves {
                self.undo.actions = moves.to_owned();
                break;
            }
            self.try_move(move_.direction)?;
        }

        Ok(())
    }

    /// Convert moves to string, including moves that have been undone.
    ///
    /// Used for loading a level.
    pub fn all_moves_to_string(&self) -> String {
        // DEBT Should be part of load (?)
        let mut result = String::with_capacity(self.undo.actions.len());
        for mv in &self.undo.actions {
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
                let foreground = if self.dynamic.worker_position == pos {
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
        let dynamic = DynamicEntities {
            crates: level.crates.clone(),
            worker_position: level.worker_position,

            empty_goals: 0,
        };

        let mut result = Self {
            columns: level.columns,
            rows: level.rows,
            background: level.background.clone(),
            dynamic,

            undo: Undo::new(),

            listeners: vec![],
        };

        result.dynamic.empty_goals = result
            .dynamic
            .crates
            .keys()
            .filter(|&&pos| result.background(pos) != &Background::Goal)
            .count();

        result
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

        let mut lvl: CurrentLevel = Level::parse(
            0,
            "####\n\
             #@ #\n\
             ####\n",
        )
        .unwrap()
        .into();
        assert_eq!(lvl.dynamic.worker_position.x, 1);
        assert_eq!(lvl.dynamic.worker_position.y, 1);

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
        let mut lvl: CurrentLevel = Level::parse(
            0,
            "#######\n\
             #.$@$.#\n\
             #######\n",
        )
        .unwrap()
        .into();
        assert_eq!(lvl.dynamic.worker_position.x, 3);
        assert_eq!(lvl.dynamic.worker_position.y, 1);
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
