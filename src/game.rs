use collection::*;
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

    pub fn save(&mut self) {
        self.collection.save();
    }

    pub fn load(&mut self, name: &str) {}
}
