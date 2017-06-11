use std::cell::Cell;
use std::time::Instant;

use backend::Position;
use texture::*;

const ANIMATION_DURATION: f32 = 0.08;

#[derive(Clone, Debug)]
pub struct Sprite {
    /// The position of the backend object represented by the sprite. If the current sprite is
    /// animate, this is the *destination*, not the source position.
    position: Position,

    /// `None` if the sprite is not moving at the moment. Otherwise, a pair of the instant the
    /// animation was started and the position it started from.
    animation: Cell<Option<(Instant, Position)>>,
}

impl Sprite {
    /// Create a static sprite at the given position.
    pub fn new(position: Position) -> Self {
        Sprite {
            position,
            animation: Cell::new(None),
        }
    }

    /// Animate the current sprite’s movement from its current position to the given position.
    pub fn move_to(&mut self, new_position: Position) {
        let old_position = self.position;
        self.position = new_position;
        self.animation.set(Some((Instant::now(), old_position)));
        // TODO What if self.animation.get() != None?
    }

    /// Create a list of vertices of two triangles making up a square on which the texture for
    /// this sprite can be drawn.
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
