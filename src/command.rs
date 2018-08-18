use direction::*;
use position::*;
use save::UpdateResponse;

type Slot = u8;
type Steps = usize;

/// Anything the user can ask the back end to do.
#[derive(Debug, Clone)]
pub enum Command {
    /// Do not do anything. This exists solely to eliminate the need of using Option<Command>.
    Nothing,

    /// Move one step in the given direction if possible.
    Move(Direction),

    /// Move as far as possible in the given direction with or without pushing crates.
    MoveAsFarAsPossible {
        direction: Direction,
        may_push_crate: bool,
    },

    /// Move as far as possible towards the given position in the same row or column while pushing
    /// crates or to any position when not pushing crates.
    MoveToPosition {
        position: Position,
        may_push_crate: bool,
    },

    /// Undo the previous move.
    Undo,

    /// Redo a move previously undone.
    Redo,

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

    /// Start recording a macro to the given slot.
    RecordMacro(Slot),

    /// Stop recording a macro and store the result.
    StoreMacro,

    /// Execute the macro stored in the given slon.
    ExecuteMacro(Slot),
}

impl Command {
    /// Does this command change the collection of macros, i.e. cannot be safely recorded in a
    /// macro?
    pub fn changes_macros(&self) -> bool {
        match *self {
            Command::RecordMacro(_) | Command::StoreMacro => true,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        match *self {
            Command::Nothing => true,
            _ => false,
        }
    }

    pub fn to_string(&self) -> String {
        use Command::*;
        match *self {
            Move(dir) => dir.to_string(),
            // TODO Find different formats for the next two cases
            MoveAsFarAsPossible {
                direction: dir,
                may_push_crate: true,
            } => format!("_{}", dir),
            MoveAsFarAsPossible { direction: dir, .. } => format!("_{}", dir),
            MoveToPosition {
                position: pos,
                may_push_crate: true,
            } => format!("[{}, {}]", pos.x, pos.y),
            MoveToPosition { position: pos, .. } => format!("({}, {})", pos.x, pos.y),
            Undo => "<".to_string(),
            Redo => ">".to_string(),
            ExecuteMacro(slot) => format!("@{}", slot),
            Nothing | ResetLevel | NextLevel | PreviousLevel | Save | LoadCollection(_)
            | RecordMacro(_) | StoreMacro => unreachable!(),
        }
    }
}

/// This encodes whatever the GUI needs to update according to the command just executed.
#[derive(Debug)]
pub enum Response {
    /// The current level has just been solved. Additionally, signify whether, and if so which,
    /// high score has been improved upon.
    LevelFinished(UpdateResponse),

    /// A new level has been loaded. The number is the rank in the current level set.
    NewLevel {
        rank: usize,
        columns: usize,
        rows: usize,
        worker_position: Position,
        worker_direction: Direction,
    },

    /// The worker was moved to the given position and facing the given direction
    MoveWorkerTo(Position, Direction),

    /// The crate with the given index was pushed from to this new position.
    MoveCrateTo(usize, Position),

    /// A macro consisting of the given number of commands has just been defined.
    MacroDefined(Steps),

    // Errors
    /// Tried to move but hit an obstacle
    CannotMove(WithCrate, Obstacle),

    /// Tried to undo when no move has been made.
    NothingToUndo,

    /// Failed to redo a move.
    NothingToRedo,

    /// Tried to go to the non-existant level 0.
    NoPreviousLevel,

    /// Cannot load next level, the current level is the last one.
    EndOfCollection,

    /// Cannot find a path when crates may be moved.
    NoPathfindingWhilePushing,
}

impl Response {
    pub fn is_error(&self) -> bool {
        use Response::*;
        match *self {
            LevelFinished(_) | NewLevel { .. } | MoveWorkerTo(..) | MoveCrateTo(..) => false,
            _ => true,
        }
    }
}

/// Did the player try to move a crate?
#[derive(Clone, Debug)]
pub struct WithCrate(pub bool);

/// What blacked a movement?
#[derive(Clone, Debug)]
pub enum Obstacle {
    Wall,
    Crate,
    // TODO multiple workers might block each other
}

#[cfg(test)]
pub fn contains_error(responses: &[Response]) -> bool {
    responses.iter().any(|x| x.is_error())
}
