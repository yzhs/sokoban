use collection::*;
use command::*;
use direction::Direction;
use position::Position;
use util::SokobanError;

#[derive(Debug)]
pub struct Game {
    pub name: String,
    pub collection: Collection,
}

impl Game {
    pub fn new(name: &str) -> Result<Self, SokobanError> {
        Ok(Game {
               name: name.into(),
               collection: Collection::load(name)?,
           })
    }

    pub fn set_collection(&mut self, name: &str) -> Result<(), SokobanError> {
        self.collection = Collection::load(name)?;
        Ok(())
    }

    pub fn execute(&mut self, cmd: Command) -> Vec<Response> {
        if let Command::LoadCollection(name) = cmd {
            error!("Loading level collection {}.", name);
            self.set_collection(&name).unwrap();
            vec![Response::NewLevel(self.collection.current_level.rank)]
        } else {
            self.collection.execute(cmd)
        }
    }

    /// Save current state.
    pub fn save(&mut self) -> Result<(), SaveError> {
        self.collection.save()
    }

    /// Load the saved state for the collection with the given name.
    pub fn load(&mut self, name: &str) -> Result<(), SokobanError> {
        self.set_collection(name)
    }

    pub fn worker_position(&self) -> Position {
        self.collection.current_level.worker_position
    }

    pub fn worker_direction(&self) -> Direction {
        self.collection.worker_direction()
    }

    pub fn columns(&self) -> usize {
        self.collection.current_level.columns()
    }

    pub fn rows(&self) -> usize {
        self.collection.current_level.rows()
    }
}
