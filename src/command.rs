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

    /// Go to the next level in the current collection if the current level has been solved.
    NextLevel,

    /// Go back a level.
    PreviousLevel,

    /// Switch to the level collection with the given name.
    LoadCollection(String),
}
