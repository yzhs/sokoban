use std::convert::TryFrom;

use collection::*;
use command::*;
use direction::Direction;
use level::Level;
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

#[derive(Debug)]
pub struct Game {
    name: String,

    /// A copy of one of the levels.
    current_level: Level,

    collection: Collection,

    /// What levels have been solved and with how many moves/pushes.
    state: CollectionState,

    /// Macros
    macros: Macros,
}

impl Game {
    pub fn new(name: &str) -> Result<Self, SokobanError> {
        let collection = Collection::parse(name)?;

        let mut result = Game {
            name: name.into(),
            current_level: collection.first_level().clone(),
            collection,
            state: CollectionState::load(name),
            macros: Macros::new(),
        };

        result.load(true);

        Ok(result)
    }

    /// Load a collection by name.
    pub fn set_collection(&mut self, name: &str) -> Result<(), SokobanError> {
        self.name = name.into();
        self.collection = Collection::parse(name)?;
        self.current_level = self.collection.first_level().clone();
        self.load(true);
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
    pub fn execute(&mut self, cmd: &Command) -> Vec<Response> {
        if let Command::LoadCollection(ref name) = *cmd {
            error!("Loading level collection {}.", name);
            self.set_collection(name).unwrap();
            vec![self.new_level()]
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
    fn execute_helper(&mut self, command: &Command, executing_macro: bool) -> Vec<Response> {
        use Command::*;

        // Record everything while recording a macro. If no macro is currently being recorded,
        // Macros::push will just do nothing.
        if !executing_macro && !command.changes_macros() && !command.is_empty() {
            self.macros.push(command);
        }

        let mut result = match *command {
            Command::Nothing => vec![],

            Move(dir) => self.current_level.try_move(dir),
            MoveAsFarAsPossible(dir, MayPushCrate(b)) => {
                self.current_level.move_until(dir, b).unwrap_or_default()
            }
            MoveToPosition(pos, MayPushCrate(b)) => self.current_level.move_to(pos, b),

            Undo => self.current_level.undo(),
            Redo => self.current_level.redo(),
            ResetLevel => vec![self.reset_level()],

            NextLevel => self.next_level().unwrap_or_default(),
            PreviousLevel => self.previous_level().unwrap_or_default(),

            Save => {
                let _ = self.save().unwrap();
                vec![]
            }

            RecordMacro(slot) => {
                self.macros.record(slot);
                vec![]
            }
            StoreMacro => {
                let len = self.macros.store();
                if len == 0 {
                    vec![]
                } else {
                    vec![Response::MacroDefined(self.macros.store())]
                }
            }
            ExecuteMacro(slot) => {
                let cmds = self.macros.get(slot).to_owned();
                let mut result = vec![];
                for cmd in &cmds {
                    result.extend(self.execute_helper(cmd, true));
                }
                result
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
                Ok(resp) => result.push(Response::LevelFinished(resp)),
                Err(e) => {
                    error!("Failed to create data file: {}", e);
                    result.push(Response::LevelFinished(UpdateResponse::FirstTimeSolved))
                }
            }
        }
        result
    }

    // Helpers for Collection::execute

    /// Replace the current level by a clean copy.
    fn reset_level(&mut self) -> Response {
        let n = self.rank();
        self.current_level = self.collection.levels()[n - 1].clone();
        self.new_level()
    }

    /// If `current_level` is finished, switch to the next level.
    fn next_level(&mut self) -> Result<Vec<Response>, NextLevelError> {
        let n = self.rank();
        let finished = self.current_level.is_finished();
        if finished {
            if n < self.collection.number_of_levels() {
                self.current_level = self.collection.levels()[n].clone();
                Ok(vec![self.new_level()])
            } else {
                Err(NextLevelError::EndOfCollection)
            }
        } else if self.state.number_of_levels() >= n && n < self.collection.number_of_levels() {
            self.current_level = self.collection.levels()[n].clone();
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
            self.current_level = self.collection.levels()[n - 2].clone();
            Ok(vec![self.new_level()])
        }
    }

    /// Load state stored on disc.
    fn load(&mut self, parse_levels: bool) {
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
                self.current_level = lvl;
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
    use command::contains_error;

    fn exec_ok(game: &mut Game, cmd: Command) -> bool {
        !contains_error(&game.execute(&cmd))
    }

    #[test]
    fn switch_levels() {
        let mut game = Game::new("test").unwrap();
        assert!(exec_ok(&mut game, Command::Move(Direction::Right)));
        assert!(exec_ok(&mut game, Command::PreviousLevel));
        assert!(exec_ok(&mut game, Command::NextLevel));
    }

    #[test]
    fn load_original() {
        use position::Position;
        use Direction::*;

        let name = "original";
        let mut game = Game::new(name).unwrap();
        assert_eq!(game.collection.number_of_levels(), 50);
        assert_eq!(game.collection.short_name(), name);

        assert!(exec_ok(&mut game, Command::Move(Up)));
        assert!(exec_ok(
            &mut game,
            Command::MoveAsFarAsPossible(Left, MayPushCrate(true)),
        ));
        let res = game.execute(&Command::Move(Left));
        assert!(contains_error(&res));

        assert!(exec_ok(&mut game, Command::ResetLevel));
        assert!(exec_ok(
            &mut game,
            Command::MoveToPosition(Position::new(8, 4), MayPushCrate(false),),
        ));
        assert_eq!(game.current_level.number_of_moves(), 7);
        assert!(exec_ok(&mut game, Command::Move(Left)));
        assert_eq!(game.current_level.number_of_pushes(), 1);

        assert_eq!(game.current_level.moves_to_string(), "ullluuuL");
        assert!(exec_ok(&mut game, Command::Undo));
        assert_eq!(game.current_level.all_moves_to_string(), "ullluuuL");
        assert_eq!(game.current_level.moves_to_string(), "ullluuu");
        assert!(exec_ok(&mut game, Command::Redo));
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
