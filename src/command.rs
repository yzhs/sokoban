use crate::direction::*;
use crate::position::*;

type Slot = u8;

/// Anything the user can ask the back end to do.
#[derive(Debug, Clone)]
pub enum Command {
    /// Do not do anything. This exists solely to eliminate the need of using Option<Command>.
    Nothing,

    Movement(Movement),
    LevelManagement(LevelManagement),
    Macro(Macro),
}

#[derive(Debug, Clone)]
pub enum Movement {
    /// Move one step in the given direction if possible. This may involve pushing a crate.
    Step { direction: Direction },

    /// Move as far as possible in the given direction without pushing crates.
    WalkTillObstacle { direction: Direction },

    /// Push a crate as far as possible in the given direction.
    PushTillObstacle { direction: Direction },

    /// Walk straight towards `position` until the worker is there or hits an obstacle.
    WalkTowards { position: Position },

    /// Push a crate straight towards `position` until the worker is there or the crate hits an
    /// obstacle.
    PushTowards { position: Position },

    /// Walk to the given position, no matter where the path takes the worker, i.e. general path
    /// finding without moving any crates.
    WalkToPosition { position: Position },

    /// Try to push the crate at position `from` to position `to`.
    MoveCrateToTarget { from: Position, to: Position },

    /// Undo the previous move.
    Undo,

    /// Redo a move previously undone.
    Redo,
}

#[derive(Debug, Clone)]
pub enum LevelManagement {
    /// Reset the current level
    ResetLevel,

    /// Go to the next level in the current collection if the current level has been solved.
    NextLevel,

    /// Go back a level.
    PreviousLevel,

    /// Save the current level’s solution if the level is solved, otherwise save the current state.
    Save,

    /// Switch to the level collection with the given name.
    LoadCollection(String),
}

#[derive(Debug, Clone)]
pub enum Macro {
    /// Start recording a macro to the given slot.
    Record(Slot),

    /// Stop recording a macro and store the result.
    Store,

    /// Execute the macro stored in the given slon.
    Execute(Slot),
}

impl Command {
    /// Does this command change the collection of macros, i.e. cannot be safely recorded in a
    /// macro?
    pub fn changes_macros(&self) -> bool {
        if let Command::Macro(Macro::Record(_)) | Command::Macro(Macro::Store) = self {
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        match *self {
            Command::Nothing => true,
            _ => false,
        }
    }

    pub fn to_string(&self) -> String {
        use crate::Command::*;
        use crate::Macro::*;
        use crate::Movement::*;

        match *self {
            Movement(ref m) => match *m {
                Step { direction } => direction.to_string(),
                // TODO Find different formats for the next two cases
                PushTillObstacle { direction: dir } => format!("_{}", dir),
                WalkTillObstacle { direction: dir } => format!("_{}", dir),
                PushTowards { position: pos } => format!("[{}, {}]", pos.x, pos.y),
                WalkTowards { position: pos } => format!("({}, {})", pos.x, pos.y),
                WalkToPosition { position: pos } => format!("({}, {})", pos.x, pos.y),
                MoveCrateToTarget { from, to } => {
                    format!("![({},{}),({},{})]", from.x, from.y, to.x, to.y)
                }
                Undo => "<".to_string(),
                Redo => ">".to_string(),
            },
            Macro(Execute(slot)) => format!("@{}", slot),
            _ => unreachable!(),
        }
    }
}

/// Did the player try to move a crate?
#[derive(Clone, Debug)]
pub struct WithCrate(pub bool);

/// What blacked a movement?
#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Obstacle {
    Wall,
    Crate,
    // TODO multiple workers might block each other
}
