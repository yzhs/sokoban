use std::cell::Cell;
use std::time::Instant;

use backend::Position;
use texture::*;

const ANIMATION_DURATION: f32 = 0.08;

#[derive(Clone, Debug)]
pub struct Sprite {
    position: Position,
    animation: Cell<Option<(Instant, Position)>>,
}

impl Sprite {
    pub fn new(position: Position) -> Self {
        Sprite {
            position,
            animation: Cell::new(None),
        }
    }

    pub fn move_to(&mut self, new_position: Position) {
        let old_position = self.position;
        self.position = new_position;
        self.animation.set(Some((Instant::now(), old_position)));
    }

    pub fn quad(&self, columns: u32, rows: u32, aspect_ratio: f32) -> Vec<Vertex> {
        match self.animation.get() {
            None => create_quad_vertices(self.position, columns, rows, aspect_ratio),
            Some((start, old)) => {
                let duration = Instant::now() - start;
                let duration_seconds = duration.as_secs() as f32 +
                                       duration.subsec_nanos() as f32 / 1.0e9;
                let lambda = duration_seconds / ANIMATION_DURATION;
                if lambda >= 1.0 {
                    self.animation.set(None);
                    self.quad(columns, rows, aspect_ratio)
                } else {
                    interpolate_quad_vertices(self.position,
                                              old,
                                              lambda,
                                              columns,
                                              rows,
                                              aspect_ratio)
                }
            }
        }
    }
}
