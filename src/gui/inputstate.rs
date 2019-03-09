use glium::glutin::{ModifiersState, VirtualKeyCode};

use crate::backend::{Command, Direction, Position};

#[derive(Default)]
pub struct InputState {
    pub recording_macro: bool,
    pub cursor_position: [f64; 2],

    pub clicked_crate: Option<Position>,
}

impl InputState {
    /// Handle key press events.
    pub fn press_to_command(&mut self, key: VirtualKeyCode, modifiers: ModifiersState) -> Command {
        use self::Command::*;
        use self::VirtualKeyCode::*;
        match key {
            // Move
            Left | Right | Up | Down => {
                let direction = key_to_direction(key);
                return if !modifiers.ctrl && !modifiers.shift {
                    Step { direction }
                } else if modifiers.ctrl && modifiers.shift {
                    Nothing
                } else if modifiers.shift {
                    WalkTillObstacle { direction }
                } else {
                    PushTillObstacle { direction }
                };
            }

            // Undo and redo
            Z if !modifiers.ctrl => {}
            U if modifiers.ctrl => {}
            U | Z if modifiers.shift => return Redo,
            U | Z => return Undo,

            // Record or execute macro
            F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 => {
                let n = key_to_num(key);
                return if self.recording_macro && modifiers.ctrl {
                    // Finish recording
                    self.recording_macro = false;
                    StoreMacro
                } else if modifiers.ctrl {
                    // Start recording
                    self.recording_macro = true;
                    RecordMacro(n)
                } else {
                    // Execute
                    ExecuteMacro(n)
                };
            }

            P => return PreviousLevel,
            N => return NextLevel,

            S if modifiers.ctrl => return Save,

            // TODO Open the main menu
            Escape => return ResetLevel,
            LAlt | LControl | LShift | LWin | RAlt | RControl | RShift | RWin => {}
            _ => error!("Unknown key: {:?}", key),
        }
        Nothing
    }
}

// Helper functions for input handling
/// Map Fn key to their index in [F1, F2, ..., F12].
fn key_to_num(key: VirtualKeyCode) -> u8 {
    use self::VirtualKeyCode::*;
    match key {
        F1 => 0,
        F2 => 1,
        F3 => 2,
        F4 => 3,
        F5 => 4,
        F6 => 5,
        F7 => 6,
        F8 => 7,
        F9 => 8,
        F10 => 9,
        F11 => 10,
        F12 => 11,
        _ => unreachable!(),
    }
}

/// Map arrow keys to the corresponding directions, panic on other keys.
fn key_to_direction(key: VirtualKeyCode) -> Direction {
    match key {
        VirtualKeyCode::Left => Direction::Left,
        VirtualKeyCode::Right => Direction::Right,
        VirtualKeyCode::Up => Direction::Up,
        VirtualKeyCode::Down => Direction::Down,
        _ => unreachable!(),
    }
}
