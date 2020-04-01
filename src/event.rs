use std::collections::HashMap;

use crate::command::*;
use crate::current_level::{BlockedEntity, FailedMove};
use crate::direction::Direction;
use crate::level::Background;
use crate::position::Position;
use crate::save::*;

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
        use crate::Event::*;
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

impl From<FailedMove> for Event {
    fn from(failed_move: FailedMove) -> Self {
        let with_crate = if let BlockedEntity::Crate = failed_move.thing_blocked {
            true
        } else {
            false
        };
        Event::CannotMove(WithCrate(with_crate), failed_move.obstacle_type)
    }
}
