use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::mpsc::Sender;

use command::{Obstacle, WithCrate};
use direction::*;
use game::Event;
use move_::Move;
use position::*;
use util::*;

/// Static part of a cell.
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
enum Foreground {
    None,
    Worker,
    Crate,
}

#[derive(Debug, Clone)]
pub struct Level {
    rank: usize,
    columns: usize,
    rows: usize,

    /// `columns * rows` cells’ backgrounds in row-major order
    pub background: Vec<Background>,

    /// Positions of all crates
    pub crates: HashMap<Position, usize>,

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

// Parse level {{{
fn char_to_cell(chr: char) -> Option<(Background, Foreground)> {
    match chr {
        '#' => Some((Background::Wall, Foreground::None)),
        ' ' => Some((Background::Empty, Foreground::None)),
        '$' => Some((Background::Floor, Foreground::Crate)),
        '@' => Some((Background::Floor, Foreground::Worker)),
        '.' => Some((Background::Goal, Foreground::None)),
        '*' => Some((Background::Goal, Foreground::Crate)),
        '+' => Some((Background::Goal, Foreground::Worker)),
        _ => None,
    }
}

struct LevelBuilder {
    rank: usize,
    columns: usize,
    rows: usize,
    background: Vec<Background>,
    crates: HashMap<Position, usize>,
    empty_goals: usize,
    worker_position: Position,
}

fn is_empty_or_comment(s: &str) -> bool {
    s.is_empty() || s.trim().starts_with(';')
}

impl LevelBuilder {
    pub fn new(rank: usize, level_string: &str) -> Result<Self, SokobanError> {
        let lines: Vec<_> = level_string
            .lines()
            .filter(|x| !is_empty_or_comment(x))
            .collect();
        let rows = lines.len();
        if rows == 0 {
            return Err(SokobanError::NoLevel(rank));
        }
        let columns = lines.iter().map(|x| x.len()).max().unwrap();
        if columns == 0 {
            return Err(SokobanError::NoLevel(rank));
        }

        let mut found_worker = false;
        let mut worker_position = Position { x: 0, y: 0 };
        let mut empty_goals = 0;
        let mut background = vec![Background::Empty; columns * rows];
        let mut crates = Vec::with_capacity(20);

        let mut goals_minus_crates = 0_i32;

        let mut found_level_description = false;
        for (y, line) in lines.iter().enumerate() {
            let mut inside = false;
            for (x, chr) in line.chars().enumerate() {
                let (bg, fg) = char_to_cell(chr).unwrap_or_else(|| {
                    panic!("Invalid character '{}' in line {}, column {}.", chr, y, x)
                });
                let index = y * columns + x;
                background[index] = bg;
                found_level_description = true;

                // Count goals still to be filled and make sure that there are exactly as many
                // goals as there are crates.
                if bg == Background::Goal && fg != Foreground::Crate {
                    empty_goals += 1;
                    goals_minus_crates += 1;
                } else if bg != Background::Goal && fg == Foreground::Crate {
                    goals_minus_crates -= 1;
                }
                if fg == Foreground::Crate {
                    crates.push(Position::new(x, y));
                }

                // Try to figure out whether a given cell is inside the walls.
                if !inside && bg.is_wall() {
                    inside = true;
                }

                if inside
                    && bg == Background::Empty
                    && index >= columns
                    && background[index - columns] != Background::Empty
                {
                    background[index] = Background::Floor;
                }

                // Find the initial worker position.
                if fg == Foreground::Worker {
                    if found_worker {
                        return Err(SokobanError::TwoWorkers(rank));
                    }
                    worker_position = Position::new(x, y);
                    found_worker = true;
                }
            }
        }
        if !found_level_description {
            return Err(SokobanError::NoLevel(rank));
        } else if !found_worker {
            return Err(SokobanError::NoWorker(rank));
        } else if goals_minus_crates != 0 {
            return Err(SokobanError::CratesGoalsMismatch(rank, goals_minus_crates));
        }

        let swap = |(a, b)| (b, a);
        let crates = crates.into_iter().enumerate().map(swap).collect();
        Ok(Self {
            rank,
            columns,
            rows,
            background,
            crates,
            empty_goals,
            worker_position,
        })
    }

    pub fn build(mut self) -> Level {
        self.correct_outside_cells();
        Level {
            rank: self.rank,
            columns: self.columns,
            rows: self.rows,
            background: self.background,
            crates: self.crates,
            empty_goals: self.empty_goals,
            worker_position: self.worker_position,

            number_of_moves: 0,
            moves: vec![],

            listeners: vec![],
        }
    }

    /// Fix the mistakes of the heuristic used in `new()` for detecting which cells are on the
    /// inside.
    fn correct_outside_cells(&mut self) {
        let columns = self.columns;

        let mut queue = VecDeque::new();
        let mut visited = vec![false; self.background.len()];
        visited[self.worker_position.to_index(columns)] = true;

        let mut inside = visited.clone();

        queue.push_back(self.worker_position);

        for crate_pos in self.crates.keys() {
            visited[crate_pos.to_index(columns)] = true;
            queue.push_back(*crate_pos);
        }

        for (i, &bg) in self.background.iter().enumerate() {
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
            if let Background::Wall = self.background[i] {
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

        for (i, bg) in self.background.iter_mut().enumerate() {
            if !inside[i] && *bg == Background::Floor {
                *bg = Background::Empty;
            }
        }
    }
}
// }}}

/// Parse level and some basic utility functions. None of these change an existing `Level`. {{{
impl Level {
    /// Parse the ASCII representation of a level.
    pub fn parse(num: usize, string: &str) -> Result<Level, SokobanError> {
        let builder = LevelBuilder::new(num + 1, string)?;
        Ok(builder.build())
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
impl Level {
    pub fn subscribe(&mut self, sender: Sender<Event>) {
        self.listeners.push(sender);
    }

    fn notify(&self, event: Event) {
        for sender in &self.listeners {
            sender.send(event.clone()).unwrap();
        }
    }

    fn move_worker(&mut self, direction: Direction) {
        let to = self.worker_position.neighbour(direction);
        self.move_worker_to(to, direction);
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
        self.notify(event);
    }

    fn move_crate(&mut self, from: Position, direction: Direction) {
        self.move_crate_to(from, from.neighbour(direction));
    }

    // NOTE We need `from` so we can findout the crate's id. That way, the user interface knows
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
        self.notify(event);
    }
}
// }}}

/// Movement, i.e. everything that *does* change the `self`.
impl Level {
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
    pub fn move_to(&mut self, to: Position, may_push_crate: bool) {
        match direction(self.worker_position, to) {
            Ok(dir) => {
                let (dx, dy) = to - self.worker_position;
                if !may_push_crate && dx.abs() + dy.abs() > 1 {
                    self.find_path(to).unwrap_or_default();
                } else {
                    // Note that this takes care of both movements of just one step and all cases
                    // in which crates may be pushed.
                    while self.move_helper(dir, may_push_crate).is_ok() {
                        if self.worker_position == to || may_push_crate && self.is_finished() {
                            break;
                        }
                    }
                }
            }
            Err(None) => {}
            Err(_) if !may_push_crate => {
                self.find_path(to);
            }
            Err(_) => self.notify(Event::NoPathfindingWhilePushing),
        }
    }

    /// Try to move in the given direction. Return an error if that is not possile.
    pub fn try_move(&mut self, direction: Direction) -> Result<(), ()> {
        self.move_helper(direction, true).map_err(|_| ())
    }

    /// Try to find a shortest path from the workers current position to `to` and execute it if one
    /// exists.
    pub fn find_path(&mut self, to: Position) -> Result<(), ()> {
        let columns = self.columns();
        let rows = self.rows();

        if self.worker_position == to || !self.is_empty(to) {
            return Ok(());
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
                    if distances[self.index(neighbour)]
                        < distances[self.index(self.worker_position)]
                    {
                        let dir = direction(self.worker_position, neighbour);
                        self.try_move(dir.unwrap());
                    }
                }
                if self.worker_position == to {
                    break;
                }
            }
        }

        Ok(())
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
            self.notify(Event::NothingToUndo);
            return false;
        } else {
            self.number_of_moves -= 1;
        }

        let direction = self.moves[self.number_of_moves].direction;
        let crate_pos = self.worker_position.neighbour(direction);
        self.move_worker(direction.reverse());

        if self.moves[self.number_of_moves].moves_crate {
            self.move_crate(crate_pos, direction.reverse());
        }

        true
    }

    /// If a move has been undone previously, redo it.
    pub fn redo(&mut self) -> bool {
        if self.moves.len() > self.number_of_moves {
            let dir = self.moves[self.number_of_moves].direction;
            self.try_move(dir);
            true
        } else {
            self.notify(Event::NothingToRedo);
            false
        }
    }

    /// Given a number of simple moves, i.e. up, down, left, right, as a strign, execute the first
    /// `number_of_moves` of them. If there are more moves than that, they can be executed using
    /// redo.
    pub fn execute_moves(&mut self, number_of_moves: usize, moves: &str) -> Result<(), ()> {
        let moves = ::move_::parse(moves).unwrap();
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

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_crate_missing() {
        let s = "@.*.*.";
        let res = Level::parse(0, s);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Level #1: #crates - #goals = 3"
        );
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
        assert_eq!(
            res.unwrap_err().to_string(),
            "More than one worker in level #1"
        );
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
        assert_eq!(res.unwrap_err().to_string(), "No worker in level #1");
    }

    #[test]
    fn test_trivial_move_1() {
        use self::Direction::*;

        let mut lvl = Level::parse(
            0,
            "####\n\
             #@ #\n\
             ####\n",
        ).unwrap();
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
        let mut lvl = Level::parse(
            0,
            "#######\n\
             #.$@$.#\n\
             #######\n",
        ).unwrap();
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

    #[test]
    fn test_empty_level() {
        let lvl = Level::parse(0, "");
        assert!(lvl.is_err());
        if let Err(SokobanError::NoLevel(1)) = lvl {
        } else {
            unreachable!();
        }
    }

    #[test]
    fn out_of_bounds_not_interior() {
        let lvl = Level::parse(
            0,
            "#######\n\
             #.$@$.#\n\
             #######\n",
        ).unwrap();
        assert!(!lvl.is_interior(Position { x: -1, y: 0 }));
        assert!(!lvl.is_interior(Position { x: 1, y: -3 }));
    }

    #[test]
    #[should_panic]
    fn invalid_char() {
        let _ = Level::parse(0, "#######\n#.$@a #\n#######\n");
    }
}
