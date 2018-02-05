use std::convert::TryFrom;
use std::io::Read;
use std::fs::File;

use command::*;
use direction::*;
use level::*;
use macros::Macros;
use save::*;
use util::*;

enum FileFormat {
    Ascii,
    Xml,
}

/// A collection of levels.
#[derive(Debug)]
pub struct Collection {
    /// The full name of the collection.
    pub name: String,

    /// The name of the file containing the level collection.
    pub short_name: String,

    pub description: Option<String>,

    number_of_levels: usize,

    /// A copy of one of the levels.
    pub current_level: Level,

    /// All levels of this collection. This variable is only written to when loading the
    /// collection.
    levels: Vec<Level>,

    /// What levels have been solved and with how many moves/pushes.
    state: CollectionState,

    /// Macros
    macros: Macros,
}

impl Collection {
    #[cfg(test)]
    pub fn from_levels(name: &str, levels: &[Level]) -> Collection {
        Collection {
            name: name.into(),
            short_name: name.into(),
            description: None,
            number_of_levels: levels.len(),
            current_level: levels[0].clone(),
            levels: levels.into(),
            state: CollectionState::new(name),
            macros: Macros::new(),
        }
    }

    /// Load a level set with the given name, whatever the format might be.
    pub fn parse(short_name: &str, parse_levels: bool) -> Result<Collection, SokobanError> {
        let mut level_path = ASSETS.clone();
        level_path.push("levels");
        level_path.push(short_name);
        level_path.set_extension("lvl");

        let (level_file, file_format) = {
            if let Ok(f) = File::open(&level_path) {
                (f, FileFormat::Ascii)
            } else {
                level_path.set_extension("slc");
                match File::open(level_path) {
                    Ok(f) => (f, FileFormat::Xml),
                    Err(e) => return Err(SokobanError::from(e)),
                }
            }
        };

        let mut collection = match file_format {
            FileFormat::Ascii => Collection::parse_lvl(short_name, level_file, parse_levels)?,
            FileFormat::Xml => Collection::parse_xml(short_name, level_file, parse_levels)?,
        };

        collection.load(parse_levels);

        Ok(collection)
    }

    /// Load a file containing a bunch of levels separated by an empty line, i.e. the usual ASCII
    /// format.
    fn parse_lvl(
        short_name: &str,
        file: File,
        parse_levels: bool,
    ) -> Result<Collection, SokobanError> {
        #[cfg(unix)]
        const EMPTY_LINE: &str = "\n\n";
        #[cfg(windows)]
        const EMPTY_LINE: &str = "\r\n\r\n";
        let eol = |c| c == '\n' || c == '\r';
        let mut file = file;

        // Read the collection’s file
        let mut content = "".to_string();
        file.read_to_string(&mut content)?;

        let level_strings: Vec<_> = content
            .split(EMPTY_LINE)
            .map(|x| x.trim_matches(&eol))
            .filter(|x| !x.is_empty())
            .collect();
        let name = level_strings[0].lines().next().unwrap();
        let description = level_strings[0]
            .splitn(1, &eol)
            .last()
            .map(|x| x.trim().to_owned());

        // Parse the individual levels
        let (num, levels) = {
            if parse_levels {
                let lvls = level_strings[1..]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| Level::parse(i, l.trim_matches(&eol)))
                    .collect::<Result<Vec<_>, _>>()?;
                (lvls.len(), lvls)
            } else {
                (level_strings.len() - 1, vec![])
            }
        };

        Ok(Collection {
            name: name.to_string(),
            short_name: short_name.to_string(),
            description,
            number_of_levels: num,
            current_level: if parse_levels {
                levels[0].clone()
            } else {
                Level::parse(0, "###\n#@#\n###").unwrap()
            },
            levels,
            state: CollectionState::new(short_name),
            macros: Macros::new(),
        })
    }

    /// Load a level set in the XML-based .slc format.
    fn parse_xml(
        short_name: &str,
        file: File,
        parse_levels: bool,
    ) -> Result<Collection, SokobanError> {
        use quick_xml::reader::Reader;
        use quick_xml::events::Event;

        enum State {
            Nothing,
            Title,
            Description,
            Email,
            Url,
            Line,
        }

        let file = ::std::io::BufReader::new(file);
        let mut reader = Reader::from_reader(file);

        let mut state = State::Nothing;

        // Collection attributes
        let mut title = String::new();
        let mut description = String::new();
        let mut email = String::new();
        let mut url = String::new();
        let mut levels = vec![];

        // Level attributes
        let mut num = 0;
        let mut level_lines = String::new();

        let mut buf = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name() {
                    b"Title" => {
                        state = State::Title;
                        title.clear();
                    }
                    b"Description" => state = State::Description,
                    b"Email" => state = State::Email,
                    b"Url" => state = State::Url,
                    b"Level" => level_lines.clear(),
                    b"L" => state = State::Line,
                    _ => {}
                },

                Ok(Event::End(e)) => match e.name() {
                    b"Title" | b"Description" | b"Email" | b"Url" => state = State::Nothing,
                    b"Level" => {
                        if parse_levels {
                            levels.push(Level::parse(num, &level_lines)?);
                        }
                        num += 1;
                    }
                    b"L" => {
                        state = State::Nothing;
                        level_lines.push('\n');
                    }
                    _ => {}
                },

                Ok(Event::Text(e)) => {
                    let s = e.unescape_and_decode(&reader).unwrap();
                    match state {
                        State::Nothing => {}
                        State::Title => title.push_str(&s),
                        State::Description => description.push_str(&s),
                        State::Email => email.push_str(&s),
                        State::Url => url.push_str(&s),
                        State::Line => level_lines.push_str(&s),
                    }
                }

                Ok(Event::Eof) => break,

                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                _ => {}
            }
        }

        Ok(Collection {
            name: title,
            short_name: short_name.to_string(),
            description: if description.is_empty() {
                None
            } else {
                Some(description)
            },
            number_of_levels: num,
            current_level: if parse_levels {
                levels[0].clone()
            } else {
                Level::parse(0, "###\n#@#\n###").unwrap()
            },
            levels,
            state: CollectionState::new(short_name),
            macros: Macros::new(),
        })
    }

    // Accessor methods

    /// Is the current level the last one in this collection?
    pub fn end_of_collection(&self) -> bool {
        self.current_level.rank == self.levels.len()
    }

    pub fn number_of_levels(&self) -> usize {
        self.number_of_levels
    }

    pub fn number_of_solved_levels(&self) -> usize {
        self.state.levels_finished()
    }

    pub fn is_solved(&self) -> bool {
        self.state.collection_solved
    }

    /// Find out which direction the worker is currently facing.
    pub fn worker_direction(&self) -> Direction {
        self.current_level.worker_direction()
    }
}

impl Collection {
    /// Execute whatever command we get from the frontend.
    pub fn execute(&mut self, command: &Command) -> Vec<Response> {
        self.execute_helper(command, false)
    }

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
            if self.current_level.rank == self.levels.len() {
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
        let n = self.current_level.rank;
        self.current_level = self.levels[n - 1].clone();
        Response::ResetLevel
    }

    /// If `current_level` is finished, switch to the next level.
    fn next_level(&mut self) -> Result<Vec<Response>, NextLevelError> {
        let n = self.current_level.rank;
        let finished = self.current_level.is_finished();
        if finished {
            if n < self.levels.len() {
                self.current_level = self.levels[n].clone();
                Ok(vec![Response::NewLevel(n + 1)])
            } else {
                Err(NextLevelError::EndOfCollection)
            }
        } else if self.state.levels.len() >= n && n < self.levels.len() {
            self.current_level = self.levels[n].clone();
            Ok(vec![Response::NewLevel(n + 1)])
        } else {
            Err(NextLevelError::LevelNotFinished)
        }
    }

    /// Go to the previous level unless this is already the first level in this collection.
    fn previous_level(&mut self) -> Result<Vec<Response>, ()> {
        let n = self.current_level.rank;
        if n < 2 {
            Err(())
        } else {
            self.current_level = self.levels[n - 2].clone();
            Ok(vec![Response::NewLevel(n - 1)])
        }
    }

    /// Load state stored on disc.
    fn load(&mut self, parse_levels: bool) {
        let state = CollectionState::load(&self.short_name);
        if parse_levels && !state.collection_solved {
            let n = state.levels_finished();
            let mut lvl = self.levels[n].clone();
            if n < state.levels.len() {
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
        };
        self.state = state;
    }

    /// Save the state of this collection including the state of the current level.
    fn save(&mut self) -> Result<UpdateResponse, SaveError> {
        // TODO self should not be mut
        let rank = self.current_level.rank;
        let level_state = match Solution::try_from(&self.current_level) {
            Ok(soln) => LevelState::new_solved(self.current_level.rank, soln),
            _ => LevelState::new_unsolved(&self.current_level),
        };
        let response = self.state.update(rank - 1, level_state);

        self.state.save(&self.short_name)?;
        Ok(response)
    }
}

#[derive(Debug)]
pub enum NextLevelError {
    /// Tried to move to the next levels when the current one has not been solved.
    LevelNotFinished,

    /// Cannot move past the last level of a collection.
    EndOfCollection,
}

#[cfg(test)]
mod test {
    use super::*;
    use command::contains_error;

    fn exec_ok(col: &mut Collection, cmd: Command) -> bool {
        !contains_error(&col.execute(&cmd))
    }

    #[test]
    fn load_test_collections() {
        assert!(Collection::parse("test_2", true).is_ok());
        assert!(Collection::parse("test_2", false).is_ok());
        assert!(Collection::parse("test3iuntrenutineaniutea", true).is_err());
        assert!(Collection::parse("test3iuntrenutineaniutea", false).is_err());
    }

    #[test]
    fn switch_levels() {
        let mut col = Collection::parse("test", true).unwrap();
        assert!(exec_ok(&mut col, Command::Move(Direction::Right)));
        assert!(exec_ok(&mut col, Command::PreviousLevel));
        assert!(exec_ok(&mut col, Command::NextLevel));
    }

    #[test]
    fn load_original() {
        use Direction::*;
        use position::Position;

        let name = "original";
        let mut col = Collection::parse(name, true).unwrap();
        assert_eq!(col.number_of_levels(), 50);
        assert_eq!(col.short_name, name);

        assert!(exec_ok(&mut col, Command::Move(Up)));
        assert!(exec_ok(
            &mut col,
            Command::MoveAsFarAsPossible(Left, MayPushCrate(true)),
        ));
        let res = col.execute(&Command::Move(Left));
        assert!(contains_error(&res));

        assert!(exec_ok(&mut col, Command::ResetLevel));
        assert!(exec_ok(
            &mut col,
            Command::MoveToPosition(Position::new(8, 4), MayPushCrate(false),),
        ));
        assert_eq!(col.current_level.number_of_moves(), 7);
        assert!(exec_ok(&mut col, Command::Move(Left)));
        assert_eq!(col.current_level.number_of_pushes(), 1);

        assert_eq!(col.current_level.moves_to_string(), "ullluuuL");
        assert!(exec_ok(&mut col, Command::Undo));
        assert_eq!(col.current_level.all_moves_to_string(), "ullluuuL");
        assert_eq!(col.current_level.moves_to_string(), "ullluuu");
        assert!(exec_ok(&mut col, Command::Redo));
        assert_eq!(col.current_level.number_of_pushes(), 1);
    }

}
