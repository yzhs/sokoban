use backend::direction::Direction;

/// This structure contains everything needed to do or undo a Sokoban move.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    /// Was a crate moved?
    pub moves_crate: bool,

    /// Where was the move directed?
    pub direction: Direction,
}
