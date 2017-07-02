use std::convert::TryFrom;
use std::io::{self, Read};
use std::fmt;
use std::error;
use std::fs::File;
use std::path::PathBuf;

use command::*;
use direction::*;
use level::*;
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

    /// A copy of one of the levels.
    pub current_level: Level,

    /// All levels of this collection. This variable is only written to when loading the
    /// collection.
    levels: Vec<Level>,

    /// What levels have been solved and with how many moves/pushes.
    saved: CollectionState,
}

impl Collection {
    /// Load a level set with the given name, whatever the format might be.
    pub fn load(short_name: &str) -> Result<Collection, SokobanError> {
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

        match file_format {
            FileFormat::Ascii => Collection::load_lvl(short_name, level_file),
            FileFormat::Xml => Collection::load_xml(short_name, level_file),
        }
    }

    /// Load a file containing a bunch of levels separated by an empty line, i.e. the usual ASCII
    /// format.
    pub fn load_lvl(short_name: &str, file: File) -> Result<Collection, SokobanError> {
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
            .splitn(1, |c| c == '\n' || c == '\r')
            .last()
            .map(|x| x.trim().to_owned());

        // Parse the individual levels
        let levels = level_strings[1..]
            .iter()
            .enumerate()
            .map(|(i, l)| Level::parse(i, l.trim_matches(&eol)))
            .collect::<Result<Vec<_>, _>>()?;

        // Try to load the collection’s status
        let state = CollectionState::load(short_name);
        let current_level = if state.collection_solved {
            levels[0].clone()
        } else {
            let n = state.levels_finished();
            let mut lvl = levels[n].clone();
            if n < state.levels.len() {
                if let LevelState::Started {
                           number_of_moves,
                           ref moves,
                           ..
                       } = state.levels[n] {
                    lvl.execute_moves(number_of_moves, moves);
                }
            }
            lvl
        };

        let result = Collection {
            name: name.to_string(),
            short_name: short_name.to_string(),
            description,
            current_level,
            levels,
            saved: state,
        };
        Ok(result)
    }

    /// Load a level set in the XML-based .slc format.
    pub fn load_xml(short_name: &str, file: File) -> Result<Collection, SokobanError> {
        use xml::reader::{EventReader, XmlEvent};

        enum State {
            Nothing,
            Title,
            Description,
            Email,
            Url,
            Line,
        }

        let file = ::std::io::BufReader::new(file);
        let parser = EventReader::new(file);

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

        for e in parser {
            match e {
                Ok(XmlEvent::StartElement { ref name, .. }) => {
                    match name.local_name.as_ref() {
                        "Title" => {
                            state = State::Title;
                            title.clear();
                        }
                        "Description" => state = State::Description,
                        "Email" => state = State::Email,
                        "Url" => state = State::Url,
                        "Level" => level_lines.clear(),
                        "L" => state = State::Line,
                        _ => {}
                    }
                }

                Ok(XmlEvent::EndElement { ref name }) => {
                    match name.local_name.as_ref() {
                        "Title" | "Description" | "Email" | "Url" => state = State::Nothing,
                        "Level" => {
                            levels.push(Level::parse(num, &level_lines)?);
                            num += 1;
                        }
                        "L" => {
                            state = State::Nothing;
                            level_lines.push('\n');
                        }
                        _ => {}
                    }
                }
                Ok(XmlEvent::Characters(s)) => {
                    match state {
                        State::Nothing => {}
                        State::Title => title.push_str(&s),
                        State::Description => description.push_str(&s),
                        State::Email => email.push_str(&s),
                        State::Url => url.push_str(&s),
                        State::Line => level_lines.push_str(&s),
                    }
                }

                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }

                Ok(XmlEvent::StartDocument { .. }) |
                Ok(XmlEvent::EndDocument { .. }) |
                Ok(XmlEvent::ProcessingInstruction { .. }) |
                Ok(XmlEvent::CData(_)) |
                Ok(XmlEvent::Comment(_)) |
                Ok(XmlEvent::Whitespace(_)) => {}
            }
        }

        let state = CollectionState::load(short_name);
        let current_level = if state.collection_solved {
            levels[0].clone()
        } else {
            let n = state.levels_finished();
            let mut lvl = levels[n].clone();
            if n < state.levels.len() {
                if let LevelState::Started {
                           number_of_moves,
                           ref moves,
                           ..
                       } = state.levels[n] {
                    lvl.execute_moves(number_of_moves, moves);
                }
            }
            lvl
        };


        Ok(Collection {
               name: title,
               short_name: short_name.to_string(),
               description: if description.is_empty() {
                   Option::None
               } else {
                   Option::Some(description)
               },
               current_level,
               levels,
               saved: state,
           })
    }

    // Accessor methods

    /// Is the current level the last one in this collection?
    pub fn end_of_collection(&self) -> bool {
        self.current_level.rank == self.levels.len()
    }

    pub fn number_of_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn number_of_solved_levels(&self) -> usize {
        self.saved.levels_finished()
    }

    pub fn is_solved(&self) -> bool {
        self.saved.collection_solved
    }

    /// Find out which direction the worker is currently facing.
    pub fn worker_direction(&self) -> Direction {
        self.current_level.worker_direction()
    }
}


impl Collection {
    /// Execute whatever command we get from the frontend.
    pub fn execute(&mut self, command: Command) -> Vec<Response> {
        use Command::*;
        let mut result = match command {
            Command::Nothing => vec![],
            Move(dir) => self.current_level.try_move(dir),
            MoveAsFarAsPossible(dir, MayPushCrate(b)) => {
                self.current_level
                    .move_until(dir, b)
                    .unwrap_or_default()
            }
            MoveToPosition(pos, MayPushCrate(b)) => self.current_level.move_to(pos, b),
            Undo => self.current_level.undo(),
            Redo => self.current_level.redo(),
            ResetLevel => vec![self.reset_level()],
            NextLevel => self.next_level().unwrap_or_default(),
            PreviousLevel => self.previous_level().unwrap_or_default(),
            LoadCollection(_) => unreachable!(),
            Save => {
                let _ = self.save().unwrap();
                vec![]
            }
        };
        if self.current_level.is_finished() {
            if self.current_level.rank == self.levels.len() {
                self.saved.collection_solved = true;
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
        } else if self.saved.levels.len() >= n && n < self.levels.len() {
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

    /// Save the state of this collection including the state of the current level.
    fn save(&mut self) -> Result<UpdateResponse, SaveError> {
        // TODO self should not be mut
        let rank = self.current_level.rank;
        let level_state = match Solution::try_from(&self.current_level) {
            Ok(soln) => LevelState::new_solved(self.current_level.rank, soln),
            _ => LevelState::new_unsolved(&self.current_level),
        };
        let response = self.saved.update(rank - 1, level_state);

        // If no rank was given in the JSON file, set it.
        if self.saved.levels[0].rank() == 0 {
            for (i, lvl) in self.saved.levels.iter_mut().enumerate() {
                lvl.set_rank(i + 1);
            }
        }

        let mut path = PathBuf::new();
        path.push(&self.short_name);
        path.set_extension("json");
        match File::create(DATA_DIR.join(path.as_path())) {
            Err(e) => Err(SaveError::from(e)),
            Ok(file) => {
                ::serde_json::to_writer(file, &self.saved)
                    .map_err(SaveError::from)?;
                Ok(response)
            }
        }
    }
}

#[derive(Debug)]
pub enum NextLevelError {
    /// Tried to move to the next levels when the current one has not been solved.
    LevelNotFinished,

    /// Cannot move past the last level of a collection.
    EndOfCollection,
}

#[derive(Debug)]
pub enum SaveError {
    FailedToCreateFile(io::Error),
    FailedToWriteFile(::serde_json::Error),
}

impl error::Error for SaveError {
    fn description(&self) -> &str {
        use SaveError::*;
        match *self {
            FailedToCreateFile(_) => "Failed to create file",
            FailedToWriteFile(_) => "Failed to serialize to file",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        use SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => e.cause(),
            FailedToWriteFile(ref e) => e.cause(),
        }
    }
}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        SaveError::FailedToCreateFile(e)
    }
}
impl From<::serde_json::Error> for SaveError {
    fn from(e: ::serde_json::Error) -> Self {
        SaveError::FailedToWriteFile(e)
    }
}

impl fmt::Display for SaveError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use SaveError::*;
        match *self {
            FailedToCreateFile(ref e) => write!(fmt, "Failed to create file: {}", e),
            FailedToWriteFile(ref e) => write!(fmt, "Failed to write file: {}", e),
        }
    }
}
