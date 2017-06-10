use backend::Position;
use texture::{Vertex, create_quad_vertices};

#[derive(Clone, Debug)]
pub enum Sprite {
    Static { position: Position },
    Animated {
        old_position: Position,
        new_position: Position,
    },
}

impl Sprite {
    pub fn new(position: Position) -> Self {
        Sprite::Static { position }
    }

    pub fn position(&self) -> Position {
        match *self {
            Sprite::Static { position: pos } |
            Sprite::Animated { new_position: pos, .. } => pos,
        }
    }

    pub fn move_to(&mut self, new_position: Position) {
        let old_position = self.position();
        *self = Sprite::Animated {
            old_position,
            new_position,
        };
    }

    pub fn quad(&self, columns: u32, rows: u32, aspect_ratio: f32) -> Vec<Vertex> {
        create_quad_vertices(self.position(), columns, rows, aspect_ratio)
    }
}
