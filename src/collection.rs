use std::fs::File;
use std::io::Read;

use level::*;
use util::*;

enum FileFormat {
    Ascii,
    Xml,
}

/// A collection of levels.
#[derive(Debug)]
pub struct Collection {
    /// The full name of the collection.
    name: String,

    /// The name of the file containing the level collection.
    short_name: String,

    description: Option<String>,

    number_of_levels: usize,

    /// All levels of this collection. This variable is only written to when loading the
    /// collection.
    levels: Vec<Level>,
}

impl Collection {
    #[cfg(test)]
    pub fn from_levels(name: &str, levels: &[Level]) -> Collection {
        Collection {
            name: name.into(),
            short_name: name.into(),
            description: None,
            number_of_levels: levels.len(),
            levels: levels.into(),
        }
    }

    /// Load a level set with the given name, whatever the format might be.
    pub fn parse(short_name: &str) -> Result<Collection, SokobanError> {
        Collection::parse_helper(short_name, true)
    }

    /// Figure out title, description, number of levels, etc. of a collection without parsing each
    /// level.
    pub fn parse_metadata(short_name: &str) -> Result<Collection, SokobanError> {
        Collection::parse_helper(short_name, false)
    }

    fn parse_helper(short_name: &str, parse_levels: bool) -> Result<Collection, SokobanError> {
        let mut level_path = ASSETS.clone();
        level_path.push("levels");
        level_path.push(short_name);

        let (level_file, file_format) = {
            level_path.set_extension("slc");
            match File::open(&level_path) {
                Ok(f) => (f, FileFormat::Xml),
                Err(_) => {
                    level_path.set_extension("lvl");
                    match File::open(level_path) {
                        Ok(f) => (f, FileFormat::Ascii),
                        Err(e) => return Err(SokobanError::from(e)),
                    }
                }
            }
        };

        Ok(match file_format {
            FileFormat::Ascii => Collection::parse_lvl(short_name, level_file, parse_levels)?,
            FileFormat::Xml => Collection::parse_xml(short_name, level_file, parse_levels)?,
        })
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
            levels,
        })
    }

    /// Load a level set in the XML-based .slc format.
    fn parse_xml(
        short_name: &str,
        file: File,
        parse_levels: bool,
    ) -> Result<Collection, SokobanError> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

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

                Ok(Event::Text(ref e)) => match state {
                    State::Nothing => {}
                    State::Line if !parse_levels => {}
                    _ => {
                        let s = e.unescape_and_decode(&reader).unwrap();
                        match state {
                            State::Title => title.push_str(&s),
                            State::Description => description.push_str(&s),
                            State::Email => email.push_str(&s),
                            State::Url => url.push_str(&s),
                            State::Line => level_lines.push_str(&s),
                            _ => unreachable!(),
                        }
                    }
                },

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
            levels,
        })
    }

    // Accessor methods
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn short_name(&self) -> &str {
        &self.short_name
    }

    pub fn description(&self) -> Option<&str> {
        match self.description {
            Some(ref x) => Some(&x),
            None => None,
        }
    }

    pub fn first_level(&self) -> &Level {
        &self.levels[0]
    }

    /// Get all levels. This is needed for image-to-level
    pub fn levels(&self) -> &[Level] {
        self.levels.as_ref()
    }

    pub fn number_of_levels(&self) -> usize {
        self.number_of_levels
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
    #[test]
    fn load_test_collections() {
        assert!(Collection::parse("test_2").is_ok());
        assert!(Collection::parse("test_2").is_ok());
        assert!(Collection::parse("test3iuntrenutineaniutea").is_err());
        assert!(Collection::parse("test3iuntrenutineaniutea").is_err());
    }
}
