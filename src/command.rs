use direction::*;
use position::*;


pub struct MayPushCrate(pub bool);


/// Anything the user can ask the back end to do.
pub enum Command {
    /// Do not do anything. This exists solely to eliminate the need of using Option<Command>.
    Nothing,

    /// Move one step in the given direction if possible.
    Move(Direction),

    /// Move as far as possible in the given direction with or without pushing crates.
    MoveAsFarAsPossible(Direction, MayPushCrate),

    /// Move as far as possible towards the given position in the same row or column while pushing
    /// crates or to any position when not pushing crates.
    MoveToPosition(Position, MayPushCrate),

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

    /// Switch to the level collection with the given name.
    LoadCollection(String),
}


/// This encodes whatever the GUI needs to update according to the command just executed.
#[derive(Debug)]
pub enum Response {
    /// The current level has just been solved.
    LevelFinished,

    /// A new level has been loaded. The number is the rank in the current level set.
    NewLevel(usize),

    /// The worker was moved to the given position and facing the given direction
    MoveWorkerTo(Position, Direction),

    /// The crate with the given index was pushed from to this new position.
    MoveCrateTo(usize, Position),
}
