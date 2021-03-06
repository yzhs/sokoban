use std::convert::TryFrom;
use std::sync::mpsc::{Receiver, Sender};

use crate::collection::*;
use crate::command::*;
use crate::current_level::CurrentLevel;
use crate::direction::Direction;
use crate::event::*;
use crate::level::Level;
use crate::macros::Macros;
use crate::position::Position;
use crate::save::*;
use crate::util::SokobanError;

#[derive(Debug)]
pub enum NextLevelError {
    /// Tried to move to the next levels when the current one has not been solved.
    LevelNotFinished,

    /// Cannot move past the last level of a collection.
    EndOfCollection,
}

pub struct Game {
    name: String,

    rank: usize,

    /// A copy of one of the levels.
    current_level: CurrentLevel,

    collection: Collection,

    /// What levels have been solved and with how many moves/pushes.
    state: CollectionState,

    /// Macros
    macros: Macros,

    listeners: Listeners,

    receiver: Option<Receiver<Command>>,
}

#[derive(Default)]
struct Listeners {
    moves: Vec<Sender<Event>>,
}

fn notify_helper<T: Clone + Send>(listeners: &[Sender<T>], message: &T) {
    for listener in listeners {
        listener.send(message.clone()).unwrap();
    }
}

impl Listeners {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn notify_move(&self, event: &Event) {
        notify_helper(&self.moves, event);
    }

    pub fn subscribe_moves(&mut self, listener: Sender<Event>) {
        self.moves.push(listener);
    }
}

/// Handling events
impl Game {
    pub fn subscribe_moves(&mut self, listener: Sender<Event>) {
        self.current_level.subscribe(listener.clone());
        self.listeners.subscribe_moves(listener);
    }

    pub fn listen_to(&mut self, receiver: Receiver<Command>) {
        self.receiver = Some(receiver);
    }

    fn set_current_level(&mut self, level: &Level, rank: usize) {
        self.rank = rank;
        self.current_level = level.into();
        for listener in &self.listeners.moves {
            self.current_level.subscribe(listener.clone());
        }
        self.on_load_level();
    }

    fn on_load_level(&self) {
        let rank = self.rank();
        let lvl = self.get_level(rank);
        let initial_state = Event::InitialLevelState {
            rank,
            columns: self.columns(),
            rows: self.rows(),
            background: lvl.background,
            worker_position: self.worker_position(),
            worker_direction: Direction::Left,
            crates: lvl.crates,
        };
        self.listeners.notify_move(&initial_state);
    }
}

impl Game {
    pub fn new(collection: Collection) -> Self {
        let mut result = Game {
            rank: 1,
            name: collection.short_name().to_string(),
            current_level: collection.first_level().into(),
            state: CollectionState::load(collection.short_name()),
            macros: Macros::new(),
            collection,
            listeners: Listeners::new(),
            receiver: None,
        };

        result.load_state(true);

        result
    }

    /// Load a collection by name.
    pub fn set_collection(&mut self, name: &str) -> Result<(), SokobanError> {
        self.name = name.into();
        self.collection = Collection::parse(name)?;
        let level = self.collection.first_level().clone();
        self.set_current_level(&level, 1);
        self.load_state(true);
        Ok(())
    }

    /// Execute a command from the front end. Load new collections or pass control to
    /// `Collection::execute`.
    pub fn execute(&mut self) {
        while let Ok(cmd) = {
            if let Some(ref receiver) = self.receiver {
                receiver.try_recv()
            } else {
                error!("Trying to get command but no Receiver is available");
                return;
            }
        } {
            if let Command::LevelManagement(LevelManagement::LoadCollection(ref name)) = cmd {
                info!("Loading level collection {}.", name);
                self.set_collection(name).unwrap();
            } else {
                self.execute_helper(&cmd, false)
            }
        }
    }

    /// Is the current level the last one in this collection?
    pub fn is_last_level(&self) -> bool {
        self.rank() == self.collection.number_of_levels()
    }

    // Access data concerning the current level
    /// The current level
    pub fn current_level(&self) -> &CurrentLevel {
        &self.current_level
    }

    /// The rank of the current level in the current collection.
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// The number of columns of the current level.
    pub fn columns(&self) -> usize {
        self.current_level.columns()
    }

    /// The number of rows of the current level.
    pub fn rows(&self) -> usize {
        self.current_level.rows()
    }

    pub fn crate_positions(&self) -> Vec<Position> {
        self.current_level.crate_positions()
    }

    /// Where is the worker?
    pub fn worker_position(&self) -> Position {
        self.current_level.worker_position()
    }

    /// Find out which direction the worker is currently facing.
    pub fn worker_direction(&self) -> Direction {
        self.current_level.worker_direction()
    }

    /// The number of moves performed since starting to solve this level.
    pub fn number_of_moves(&self) -> usize {
        self.current_level.number_of_moves()
    }

    /// The number of pushes performed since starting to solve this level.
    pub fn number_of_pushes(&self) -> usize {
        self.current_level.number_of_pushes()
    }

    /// The collections full name
    pub fn name(&self) -> &str {
        self.collection.name()
    }
}

impl Game {
    fn send_command_to_macros(&mut self, command: &Command, executing_macro: bool) {
        // Record everything while recording a macro. If no macro is currently being recorded,
        // Macros::push will just do nothing.
        if !executing_macro && !command.changes_macros() && !command.is_empty() {
            self.macros.push(command);
        }
    }

    fn manage_level(&mut self, command: &LevelManagement) {
        use crate::LevelManagement::*;
        let is_finished = self.current_level.is_finished();

        match *command {
            ResetLevel => self.reset_level(),
            NextLevel if !is_finished => self.next_level().unwrap(),
            PreviousLevel => self.previous_level().unwrap(),

            Save if !is_finished => {
                let _ = self.save().unwrap();
            }

            // This is handled inside Game and never passed to this method.
            LoadCollection(_) => unreachable!(),

            _ => {}
        };
    }

    fn execute_movement(&mut self, movement: &Movement) {
        use crate::Movement::*;

        match *movement {
            Step { direction } => self.current_level.step(direction),
            WalkTillObstacle { direction } => {
                self.current_level.move_as_far_as_possible(direction, false)
            }
            PushTillObstacle { direction } => {
                self.current_level.move_as_far_as_possible(direction, true)
            }
            WalkTowards { position } => {
                self.current_level.move_to(position, false);
            }
            PushTowards { position } => {
                self.current_level.move_to(position, true);
            }
            WalkToPosition { position } => {
                self.current_level.move_to(position, false);
            }
            MoveCrateToTarget { from, to } => {
                self.current_level.move_crate_to_target(from, to);
            }

            Undo => {
                self.current_level.undo();
            }
            Redo => {
                self.current_level.redo();
            }
        }
    }

    pub fn macro_command(&mut self, macro_command: &Macro) {
        use crate::Macro::*;

        match *macro_command {
            Execute(slot) => self.execute_macro(slot),
            Record(slot) => {
                self.macros.start_recording(slot);
            }
            Store => {
                let len = self.macros.stop_recording();
                if len != 0 {
                    self.listeners.notify_move(&Event::MacroDefined);
                }
            }
        }
    }

    /// Execute whatever command we get from the frontend.
    fn execute_helper(&mut self, command: &Command, executing_macro: bool) {
        use crate::Command::*;

        let is_finished = self.current_level.is_finished();
        if is_finished {
            if let Command::LevelManagement(cmd) = command {
                self.manage_level(cmd);
            }
        } else {
            self.send_command_to_macros(command, executing_macro);

            match *command {
                Nothing => {}
                Movement(ref movement) => self.execute_movement(movement),
                LevelManagement(ref level_management) => self.manage_level(level_management),
                Macro(ref m) => self.macro_command(m),
            }
        }

        if self.current_level.is_finished() {
            if self.rank() == self.collection.number_of_levels() {
                self.state.collection_solved = true;
            }
            if !is_finished {
                let len = self.macros.stop_recording();
                if len != 0 {
                    self.listeners.notify_move(&Event::MacroDefined);
                }
            }

            // TODO Emit the events in one of the move() functions?
            // Save information on old level
            match self.save() {
                Ok(resp) => self.listeners.notify_move(&Event::LevelFinished(resp)),
                Err(e) => {
                    error!("Failed to create data file: {}", e);
                    self.listeners
                        .notify_move(&Event::LevelFinished(UpdateResponse::FirstTimeSolved));
                }
            }
        }
    }

    fn execute_macro(&mut self, slot: u8) {
        // NOTE We have to clone the commands so we can borrow self mutably in the loop.
        let cmds = self.macros.get(slot).to_owned();
        cmds.iter().for_each(|cmd| self.execute_helper(cmd, true));
    }

    // Helpers for Collection::execute

    fn get_level(&self, rank: usize) -> Level {
        self.collection.levels()[rank - 1].clone()
    }

    /// Replace the current level by a clean copy.
    fn reset_level(&mut self) {
        let current_level = self.get_level(self.rank());
        self.set_current_level(&current_level, self.rank);
    }

    /// If `current_level` is finished, switch to the next level.
    fn next_level(&mut self) -> Result<(), NextLevelError> {
        let n = self.rank();

        let is_last_level = n >= self.collection.number_of_levels();
        let current_level_is_solved_now = self.current_level.is_finished();
        let current_level_has_been_solved_before = n <= self.state.number_of_levels();

        if !is_last_level && (current_level_is_solved_now || current_level_has_been_solved_before) {
            let next_level = self.get_level(n + 1);
            self.set_current_level(&next_level, n + 1);
            Ok(())
        } else if is_last_level {
            Err(NextLevelError::EndOfCollection)
        } else {
            Err(NextLevelError::LevelNotFinished)
        }
    }

    /// Go to the previous level unless this is already the first level in this collection.
    fn previous_level(&mut self) -> Result<(), ()> {
        let n = self.rank();
        if n < 2 {
            Err(())
        } else {
            let previous_level = self.get_level(n - 1);
            self.set_current_level(&previous_level, n - 1);
            Ok(())
        }
    }

    /// Load state stored on disc.
    fn load_state(&mut self, parse_levels: bool) {
        info!(
            "Loading collection state for collection={}...",
            self.collection.short_name()
        );

        // DEBT this is horrible, clean it up
        let state: CollectionState;
        if parse_levels {
            state = CollectionState::load(self.collection.short_name());
            if !state.collection_solved {
                let n = state.levels_finished();

                let lvl = self.get_level(n + 1);
                self.set_current_level(&lvl, n + 1);
                self.rank = n + 1;

                if n < state.number_of_levels() {
                    if let LevelState::Started {
                        number_of_moves,
                        ref moves,
                        ..
                    } = state.levels[n]
                    {
                        let is_ok = self
                            .current_level
                            .execute_moves(number_of_moves, moves)
                            .is_ok();
                        assert!(is_ok);
                    }
                }
            }
        } else {
            state = CollectionState::load_stats(self.collection.short_name());
        }
        self.state = state;

        info!(
            "Successfully loaded collection state for collection={}: currently at level {:?}",
            self.collection.short_name(),
            self.state.levels_solved + 1,
        );
    }

    /// Save the state of this collection including the state of the current level.
    fn save(&mut self) -> Result<UpdateResponse, SaveError> {
        // TODO self should not be mut
        let rank = self.rank();
        let level_state = match Solution::try_from(&self.current_level) {
            Ok(soln) => LevelState::new_solved(soln),
            _ => LevelState::new_unsolved(&self.current_level),
        };
        let response = self.state.update(rank - 1, level_state);

        self.state.save(self.collection.short_name())?;
        Ok(response)
    }

    pub fn is_solved(&self) -> bool {
        self.state.collection_solved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver};

    fn exec_ok(game: &mut Game, receiver: &Receiver<Event>, cmd: Command) -> bool {
        game.execute_helper(&cmd, false);
        let mut found_some_event = false;

        while let Ok(event) = receiver.try_recv() {
            found_some_event = true;
            if event.is_error() {
                return false;
            }
        }

        found_some_event
    }

    fn setup_game(name: &str) -> (Game, Receiver<Event>) {
        let mut game = Game::new(Collection::parse(name).unwrap());
        let (sender, receiver) = channel();
        game.subscribe_moves(sender);
        (game, receiver)
    }

    #[test]
    fn load_original() {
        use crate::position::Position;
        use crate::Direction::*;

        let name = "original";
        let (mut game, receiver) = setup_game(name);
        assert_eq!(game.collection.number_of_levels(), 50);
        assert_eq!(game.collection.short_name(), name);

        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::Movement(Movement::Step { direction: Up })
        ));
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::Movement(Movement::PushTillObstacle { direction: Left }),
        ));
        assert!(!exec_ok(
            &mut game,
            &receiver,
            Command::Movement(Movement::Step { direction: Left })
        ));
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::LevelManagement(LevelManagement::ResetLevel)
        ));
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::Movement(Movement::WalkToPosition {
                position: Position::new(8_usize, 4),
            }),
        ));
        assert_eq!(game.current_level.number_of_moves(), 7);
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::Movement(Movement::Step { direction: Left })
        ));
        assert_eq!(game.current_level.number_of_pushes(), 1);

        assert_eq!(game.current_level.moves_to_string(), "ullluuuL");

        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::Movement(Movement::Undo)
        ));

        assert_eq!(game.current_level.all_moves_to_string(), "ullluuuL");
        assert_eq!(game.current_level.moves_to_string(), "ullluuu");
        assert_eq!(game.current_level.number_of_pushes(), 0);
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::Movement(Movement::Redo)
        ));
        assert_eq!(game.current_level.number_of_pushes(), 1);
    }

    fn create_game() -> Game {
        const LARGE_EMPTY_LEVEL: &str = r#"
#########################################
#                                    #$.#
#                                    ####
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                 @                  #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
#                                    #
######################################
"#;
        const NAME: &str = "Test";
        let lvl = Level::parse(0, LARGE_EMPTY_LEVEL).unwrap();
        let collection = Collection::from_levels(NAME, &[lvl.clone()]);
        Game {
            rank: 1,
            name: "LARGE_EMPTY_LEVEL".into(),
            collection,
            macros: Macros::new(),
            state: CollectionState::new(""),
            current_level: lvl.into(),
            listeners: Listeners::new(),
            receiver: None,
        }
    }

    #[quickcheck]
    fn prop_move_undo(mut move_dirs: Vec<Direction>) -> bool {
        let mut game = create_game();
        let lvl = game.current_level().clone();
        move_dirs.truncate(10);

        let num_moves = move_dirs.len();
        for direction in move_dirs {
            game.execute_helper(&Command::Movement(Movement::Step { direction }), false);
        }
        for _ in 0..num_moves {
            game.execute_helper(&Command::Movement(Movement::Undo), false);
        }

        let current_lvl = game.current_level();
        current_lvl.worker_position() == lvl.worker_position()
            && current_lvl.number_of_moves() == lvl.number_of_moves()
    }

    #[test]
    fn test_undo() {
        let mut game = create_game();
        let worker_position = game.worker_position();
        let number_of_moves = game.number_of_moves();

        game.execute_helper(
            &Command::Movement(Movement::Step {
                direction: Direction::Down,
            }),
            false,
        );
        game.execute_helper(&Command::Movement(Movement::Undo), false);

        let current_lvl = game.current_level();

        assert_eq!(current_lvl.worker_position(), worker_position);
        assert_eq!(current_lvl.number_of_moves(), number_of_moves);
    }
}
