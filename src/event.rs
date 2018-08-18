use std::collections::HashMap;

use command::*;
use direction::Direction;
use level::Background;
use position::Position;
use save::*;

#[derive(Clone, Debug)]
pub enum Event {
    InitialLevelState {
        rank: usize,
        columns: usize,
        rows: usize,
        background: Vec<Background>,
        worker_position: Position,
        worker_direction: Direction,
        crates: HashMap<Position, usize>,
    },
    MoveWorker {
        from: Position,
        to: Position,
        direction: Direction,
    },
    MoveCrate {
        id: usize,
        from: Position,
        to: Position,
    },
    NothingToRedo,
    NothingToUndo,
    LevelFinished(UpdateResponse),
    EndOfCollection,

    MacroDefined,

    NoPathfindingWhilePushing,
    CannotMove(WithCrate, Obstacle),
    NoPathFound,
}

#[cfg(test)]
impl Event {
    pub(crate) fn is_error(&self) -> bool {
        use Event::*;
        match self {
            InitialLevelState { .. }
            | MoveWorker { .. }
            | MoveCrate { .. }
            | LevelFinished(_)
            | EndOfCollection
            | MacroDefined => false,
            _ => true,
        }
    }
}
