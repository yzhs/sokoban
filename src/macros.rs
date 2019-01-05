use crate::command::Command;

/// A collection of macros, one for each of the F? keys, together with methods for recording and
/// accessing them.
#[derive(Debug)]
pub struct Macros {
    /// If a macro is currently being recorded, this is where its commands are stored.
    tmp: Vec<Command>,

    /// Where the macro that is currently being recorded is supposed to be stored. If this is
    /// `None`, that means that no macro is being recorded right now.
    target_slot: Option<u8>,

    /// The macros available to the user at the moment.
    slots: [Vec<Command>; 12],
}

#[cfg_attr(feature = "cargo-clippy", allow(new_without_default_derive))]
impl Macros {
    pub fn new() -> Self {
        Macros {
            tmp: vec![],
            target_slot: None,
            slots: Default::default(),
        }
    }

    /// Select the target slot.
    pub fn start_recording(&mut self, slot: u8) {
        // In case we were already recording a macro, store it. In addition, `self.tmp` is
        // cleared.
        self.stop_recording();
        self.target_slot = Some(slot);
    }

    /// Append a command to the macro currently being recorded. Return true if and only if a
    /// target slot has been selected, that is, if `start_recording` has been called before.
    pub fn push(&mut self, cmd: &Command) -> bool {
        if self.target_slot.is_some() {
            // TODO We currently unroll macros to prevent any recursive calls. Should we allow some?
            // TODO handle Undo/Redo?
            self.tmp.push(cmd.clone());
            true
        } else {
            false
        }
    }

    /// We are done recording the macro and can store it in the desired slot.
    pub fn stop_recording(&mut self) -> usize {
        if let Some(slot) = self.target_slot {
            let tmp = self.tmp.clone();
            self.tmp.clear();
            let len = tmp.len();
            self.slots[slot as usize] = tmp;
            self.target_slot = None;
            info!("Storing macro {}: {}", slot + 1, self.to_string(slot));
            len
        } else {
            0
        }
    }

    /// Retrieve the macro stored at the given slot.
    pub fn get(&self, slot: u8) -> &[Command] {
        if self.target_slot == Some(slot) {
            &[]
        } else {
            self.slots[slot as usize].as_ref()
        }
    }

    pub fn to_string(&self, slot: u8) -> String {
        let mut result = "".to_string();
        for cmd in self.slots[slot as usize].iter().filter(|&c| !c.is_empty()) {
            result.push_str(&cmd.to_string());
        }
        result
    }
}
