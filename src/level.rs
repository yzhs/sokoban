pub mod builder;

use std::{
    collections::HashMap,
    fmt,
    sync::mpsc::Sender,
};

use crate::direction::*;
use crate::event::Event;
use crate::level::builder::{Foreground, LevelBuilder};
use crate::move_::Move;
use crate::position::*;
use crate::util::*;

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

#[derive(Debug, Clone)]
pub struct Level {
    pub rank: usize,
    pub columns: usize,
    pub rows: usize,

    /// `columns * rows` cells’ backgrounds in row-major order
    pub background: Vec<Background>,

    /// Positions of all crates
    pub crates: HashMap<Position, usize>,

    /// The number of goals that have to be filled to solve the level
    pub empty_goals: usize,

    /// Where the worker is at the moment
    pub worker_position: Position,

    /// The sequence of moves performed so far. Everything after the first number_of_moves moves is
    /// used to redo moves, i.e. undoing a previous undo operation.
    pub moves: Vec<Move>,

    /// This describes how many moves have to be performed to arrive at the current state.
    pub number_of_moves: usize,

    pub listeners: Vec<Sender<Event>>,
}

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

    fn in_bounds(&self, pos: Position) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < self.columns() as isize && pos.y < self.rows() as isize
    }

    /// Is there a crate at the given position?
    fn is_crate(&self, pos: Position) -> bool {
        self.crates.get(&pos).is_some()
    }

    pub fn is_outside(&self, pos: Position) -> bool {
        !self.in_bounds(pos) || *self.background(pos) == Background::Empty
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
        )
        .unwrap();
        assert!(!lvl.is_interior(Position { x: -1, y: 0 }));
        assert!(!lvl.is_interior(Position { x: 1, y: -3 }));
    }

    #[test]
    #[should_panic]
    fn invalid_char() {
        let _ = Level::parse(0, "#######\n#.$@a #\n#######\n");
    }
}
