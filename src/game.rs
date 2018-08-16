use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::mpsc::Sender;

use collection::*;
use command::*;
use direction::Direction;
use level::{Background, Level};
use macros::Macros;
use position::Position;
use save::*;
use util::SokobanError;

#[derive(Debug)]
pub enum NextLevelError {
    /// Tried to move to the next levels when the current one has not been solved.
    LevelNotFinished,

    /// Cannot move past the last level of a collection.
    EndOfCollection,
}

pub struct Game {
    name: String,

    /// A copy of one of the levels.
    current_level: Level,

    collection: Collection,

    /// What levels have been solved and with how many moves/pushes.
    state: CollectionState,

    /// Macros
    macros: Macros,

    listener: Option<Sender<Event>>,
}

#[derive(Debug)]
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

    MacroDefined(usize),

    NoPathfindingWhilePushing,
    CannotMove(WithCrate, Obstacle),
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
            | MacroDefined(_) => false,
            _ => true,
        }
    }
}

/// Handling events
impl Game {
    pub fn subscribe(&mut self, sender: Sender<Event>) {
        self.current_level.subscribe(sender.clone());
        self.listener = Some(sender);
    }

    fn notify(&self, event: Event) {
        if let Some(ref sender) = self.listener {
            sender.send(event).unwrap();
        }
    }

    fn set_level(&mut self, level: Level) {
        self.current_level = level;
        if let Some(ref sender) = self.listener {
            self.current_level.subscribe(sender.clone());
        }
        self.on_load_level();
    }

    fn on_load_level(&self) {
        if let Some(ref sender) = self.listener {
            let initial_state = Event::InitialLevelState {
                rank: self.rank(),
                columns: self.columns(),
                rows: self.rows(),
                background: self.current_level.background.clone(),
                worker_position: self.worker_position(),
                worker_direction: Direction::Left,
                crates: self.current_level.crates.clone(),
            };
            sender.send(initial_state).unwrap();
        }
    }
}

impl Game {
    pub fn load(name: &str) -> Result<Self, SokobanError> {
        Collection::parse(name).map(Game::new)
    }

    fn new(collection: Collection) -> Self {
        let mut result = Game {
            name: collection.short_name().to_string(),
            current_level: collection.first_level().clone(),
            state: CollectionState::load(collection.short_name()),
            macros: Macros::new(),
            collection,
            listener: None,
        };

        result.load_state(true);

        result
    }

    /// Load a collection by name.
    pub fn set_collection(&mut self, name: &str) -> Result<(), SokobanError> {
        self.name = name.into();
        self.collection = Collection::parse(name)?;
        let level = self.collection.first_level().clone();
        self.load_state(true);
        self.set_level(level);
        Ok(())
    }

    fn new_level(&self) -> Response {
        Response::NewLevel {
            rank: self.rank(),
            columns: self.columns(),
            rows: self.rows(),
            worker_position: self.worker_position(),
            worker_direction: self.worker_direction(),
        }
    }

    /// Execute a command from the front end. Load new collections or pass control to
    /// `Collection::execute`.
    pub fn execute(&mut self, cmd: &Command) {
        if let Command::LoadCollection(ref name) = *cmd {
            info!("Loading level collection {}.", name);
            self.set_collection(name).unwrap();
        } else {
            self.execute_helper(cmd, false)
        }
    }

    /// Is the current level the last one in this collection?
    pub fn is_last_level(&self) -> bool {
        self.rank() == self.collection.number_of_levels()
    }

    // Access data concerning the current level
    /// The current level
    pub fn current_level(&self) -> &Level {
        &self.current_level
    }

    /// The rank of the current level in the current collection.
    pub fn rank(&self) -> usize {
        self.current_level.rank()
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
    /// Execute whatever command we get from the frontend.
    fn execute_helper(&mut self, command: &Command, executing_macro: bool) {
        use Command::*;

        // Record everything while recording a macro. If no macro is currently being recorded,
        // Macros::push will just do nothing.
        if !executing_macro && !command.changes_macros() && !command.is_empty() {
            self.macros.push(command);
        }

        match *command {
            Command::Nothing => {}

            Move(dir) => {
                self.current_level.try_move(dir);
            }
            MoveAsFarAsPossible {
                direction: dir,
                may_push_crate,
            } => {
                self.current_level
                    .move_until(dir, may_push_crate)
                    .unwrap_or_default();
            }
            MoveToPosition {
                position,
                may_push_crate,
            } => {
                self.current_level.move_to(position, may_push_crate);
            }

            Undo => {
                let _ = self.current_level.undo();
            }
            Redo => {
                let _ = self.current_level.redo();
            }
            ResetLevel => self.reset_level(),

            NextLevel => {
                self.next_level().unwrap_or_default();
            }
            PreviousLevel => {
                self.previous_level().unwrap_or_default();
            }

            Save => {
                let _ = self.save().unwrap();
            }

            RecordMacro(slot) => {
                self.macros.record(slot);
            }
            StoreMacro => {
                let len = self.macros.store();
                if len != 0 {
                    let event = Event::MacroDefined(self.macros.store());
                    self.notify(event);
                }
            }
            ExecuteMacro(slot) => {
                // NOTE We have to clone the commands so we can borrow self mutably in the loop.
                let cmds = self.macros.get(slot).to_owned();
                for cmd in &cmds {
                    self.execute_helper(cmd, true);
                }
            }

            // This is handled inside Game and never passed to this method.
            LoadCollection(_) => unreachable!(),
        };
        if self.current_level.is_finished() {
            if self.rank() == self.collection.number_of_levels() {
                self.state.collection_solved = true;
            }

            // Save information on old level
            match self.save() {
                Ok(resp) => self.notify(Event::LevelFinished(resp)),
                Err(e) => {
                    error!("Failed to create data file: {}", e);
                    self.notify(Event::LevelFinished(UpdateResponse::FirstTimeSolved));
                }
            }
        }
    }

    // Helpers for Collection::execute

    /// Replace the current level by a clean copy.
    fn reset_level(&mut self) {
        let n = self.rank();
        let level = self.collection.levels()[n - 1].clone();
        self.set_level(level);
    }

    /// If `current_level` is finished, switch to the next level.
    fn next_level(&mut self) -> Result<Vec<Response>, NextLevelError> {
        let n = self.rank();
        let finished = self.current_level.is_finished();
        if finished {
            if n < self.collection.number_of_levels() {
                let level = self.collection.levels()[n].clone();
                self.set_level(level);
                Ok(vec![self.new_level()])
            } else {
                Err(NextLevelError::EndOfCollection)
            }
        } else if self.state.number_of_levels() >= n && n < self.collection.number_of_levels() {
            let level = self.collection.levels()[n].clone();
            self.set_level(level);
            Ok(vec![self.new_level()])
        } else {
            Err(NextLevelError::LevelNotFinished)
        }
    }

    /// Go to the previous level unless this is already the first level in this collection.
    fn previous_level(&mut self) -> Result<Vec<Response>, ()> {
        let n = self.rank();
        if n < 2 {
            Err(())
        } else {
            let level = self.collection.levels()[n - 2].clone();
            self.set_level(level);
            Ok(vec![self.new_level()])
        }
    }

    /// Load state stored on disc.
    fn load_state(&mut self, parse_levels: bool) {
        let state: CollectionState;
        if parse_levels {
            state = CollectionState::load(self.collection.short_name());
            if !state.collection_solved {
                let n = state.levels_finished();
                let mut lvl = self.collection.levels()[n].clone();
                if n < state.number_of_levels() {
                    if let LevelState::Started {
                        number_of_moves,
                        ref moves,
                        ..
                    } = state.levels[n]
                    {
                        lvl.execute_moves(number_of_moves, moves);
                    }
                }
                self.set_level(lvl);
            }
        } else {
            state = CollectionState::load_stats(self.collection.short_name());
        }
        self.state = state;
    }

    /// Save the state of this collection including the state of the current level.
    fn save(&mut self) -> Result<UpdateResponse, SaveError> {
        // TODO self should not be mut
        let rank = self.rank();
        let level_state = match Solution::try_from(&self.current_level) {
            Ok(soln) => LevelState::new_solved(self.rank(), soln),
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
        game.execute(&cmd);
        while let Ok(event) = receiver.try_recv() {
            if event.is_error() {
                return false;
            }
        }
        true
    }

    fn setup_game(name: &str) -> (Game, Receiver<Event>) {
        let mut game = Game::load(name).unwrap();
        let (sender, receiver) = channel();
        game.subscribe(sender);
        (game, receiver)
    }

    #[test]
    fn switch_levels() {
        let (mut game, receiver) = setup_game("test");
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::Move(Direction::Right)
        ));
        assert!(exec_ok(&mut game, &receiver, Command::PreviousLevel));
        assert!(exec_ok(&mut game, &receiver, Command::NextLevel));
    }

    #[test]
    fn load_original() {
        use position::Position;
        use Direction::*;

        let name = "original";
        let (mut game, receiver) = setup_game("original");
        assert_eq!(game.collection.number_of_levels(), 50);
        assert_eq!(game.collection.short_name(), name);

        assert!(exec_ok(&mut game, &receiver, Command::Move(Up)));
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::MoveAsFarAsPossible {
                direction: Left,
                may_push_crate: true
            },
        ));
        assert!(!exec_ok(&mut game, &receiver, Command::Move(Left)));
        assert!(exec_ok(&mut game, &receiver, Command::ResetLevel));
        assert!(exec_ok(
            &mut game,
            &receiver,
            Command::MoveToPosition {
                position: Position::new(8_usize, 4),
                may_push_crate: false
            },
        ));
        assert_eq!(game.current_level.number_of_moves(), 7);
        assert!(exec_ok(&mut game, &receiver, Command::Move(Left)));
        assert_eq!(game.current_level.number_of_pushes(), 1);

        assert_eq!(game.current_level.moves_to_string(), "ullluuuL");
        assert!(exec_ok(&mut game, &receiver, Command::Undo));
        assert_eq!(game.current_level.all_moves_to_string(), "ullluuuL");
        assert_eq!(game.current_level.moves_to_string(), "ullluuu");
        assert!(exec_ok(&mut game, &receiver, Command::Redo));
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
            name: "LARGE_EMPTY_LEVEL".into(),
            collection,
            macros: Macros::new(),
            state: CollectionState::new(""),
            current_level: lvl,
            listener: None,
        }
    }

    #[quickcheck]
    fn prop_move_undo(mut move_dirs: Vec<Direction>) -> bool {
        let mut game = create_game();
        let lvl = game.current_level().clone();
        move_dirs.truncate(10);

        let num_moves = move_dirs.len();
        for dir in move_dirs {
            game.execute(&Command::Move(dir));
        }
        for _ in 0..num_moves {
            game.execute(&Command::Undo);
        }

        let current_lvl = game.current_level();
        current_lvl.worker_position() == lvl.worker_position()
            && current_lvl.number_of_moves() == lvl.number_of_moves()
    }
}
