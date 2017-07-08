use std::cell::Cell;
use std::time::Instant;
use std::sync::{Arc, Mutex};

use backend::{Direction, Position};
use gui::texture::*;

lazy_static! {
    /// How long it should take to animate one step.
    pub static ref ANIMATION_DURATION: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.08_f32));
}

#[derive(Clone, Debug)]
pub struct Sprite {
    /// The position of the backend object represented by the sprite. If the current sprite is
    /// animate, this is the *destination*, not the source position.
    position: Position,

    /// `None` if the sprite is not moving at the moment. Otherwise, a pair of the instant the
    /// animation was started and the position it started from.
    animation: Cell<Option<(Instant, Position)>>,

    /// What sort of tile is this?
    tile_kind: TileKind,

    /// If this is `Direction::Left`, just show the tile, otherwise rotate it until it points in
    /// the right direction.
    direction: Direction,
}

impl Sprite {
    /// Create a static sprite at the given position.
    pub fn new(position: Position, tile_kind: TileKind) -> Self {
        Sprite {
            position,
            animation: Cell::new(None),
            tile_kind,
            direction: Direction::Left,
        }
    }

    /// Animate the current sprite’s movement from its current position to the given position.
    pub fn move_to(&mut self, new_position: Position) {
        let old_position = self.position;
        self.position = new_position;
        self.animation.set(Some((Instant::now(), old_position)));
        // TODO What if self.animation.get() != None?
    }

    /// Turn the sprite in a specific direction.
    pub fn set_direction(&mut self, dir: Direction) {
        self.direction = dir;
    }

    /// Create a list of vertices of two triangles making up a square on which the texture for
    /// this sprite can be drawn.
    pub fn quad(&self, columns: u32, rows: u32, aspect_ratio: f32) -> Vec<Vertex> {
        let lambda;
        let old;
        if let Some((start, old_pos)) = self.animation.get() {
            let duration = Instant::now() - start;
            let duration_seconds = duration.as_secs() as f32 +
                                   duration.subsec_nanos() as f32 / 1.0e9;
            lambda = duration_seconds / *ANIMATION_DURATION.lock().unwrap();
            if lambda >= 1.0 {
                self.animation.set(None);
                return self.quad(columns, rows, aspect_ratio);
            }
            old = old_pos;
        } else {
            lambda = 0.0;
            old = self.position;
        }
        let new = self.position;

        let (left, right, top, bottom) = {
            let old_left = 2.0 * old.x as f32 / columns as f32 - 1.0;
            let old_right = old_left + 2.0 / columns as f32;
            let old_bottom = -2.0 * old.y as f32 / rows as f32 + 1.0;
            let old_top = old_bottom - 2.0 / rows as f32;

            let new_left = 2.0 * new.x as f32 / columns as f32 - 1.0;
            let new_right = new_left + 2.0 / columns as f32;
            let new_bottom = -2.0 * new.y as f32 / rows as f32 + 1.0;
            let new_top = new_bottom - 2.0 / rows as f32;

            (lambda * new_left + (1.0 - lambda) * old_left,
             lambda * new_right + (1.0 - lambda) * old_right,
             lambda * new_top + (1.0 - lambda) * old_top,
             lambda * new_bottom + (1.0 - lambda) * old_bottom)
        };

        lrtp_to_vertices(left, right, top, bottom, self.direction, aspect_ratio)
    }
}
