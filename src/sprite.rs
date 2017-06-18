use std::cell::Cell;
use std::time::Instant;

use backend::{Direction, Position};
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

    tile_kind: TileKind,

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
            lambda = duration_seconds / ANIMATION_DURATION;
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

        let texture_offset = match self.tile_kind {
            TileKind::Empty => 0.0,
            TileKind::Wall => 1.0,
            TileKind::Floor => 2.0,
            TileKind::Goal => 3.0,
            TileKind::Crate => 4.0,
            TileKind::Worker => 5.0,
        } / 6.0;

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

        lrtp_to_vertices_texture(left,
                                 right,
                                 top,
                                 bottom,
                                 texture_offset,
                                 self.direction,
                                 aspect_ratio)
    }
}

/// All tiles face left by default, so the worker has to turned by 90 degrees (clockwise) to face
/// up instead of left, etc.
fn direction_to_index(dir: Direction) -> usize {
    match dir {
        Direction::Left => 0,
        Direction::Down => 1,
        Direction::Right => 2,
        Direction::Up => 3,
    }
}


/// Create a vector of vertices consisting of two triangles which together form a square with the
/// given coordinates, together with texture coordinates to fill that square with a texture.
fn lrtp_to_vertices_texture(mut left: f32,
                            mut right: f32,
                            mut top: f32,
                            mut bottom: f32,
                            texture_offset: f32,
                            dir: Direction,
                            aspect_ratio: f32)
                            -> Vec<Vertex> {

    if aspect_ratio < 1.0 {
        top *= aspect_ratio;
        bottom *= aspect_ratio;
    } else {
        left /= aspect_ratio;
        right /= aspect_ratio;
    }

    let tex = [[texture_offset, 0.0],
               [texture_offset, 1.0],
               [texture_offset + 1.0 / 6.0, 1.0],
               [texture_offset + 1.0 / 6.0, 0.0]];

    let rot = direction_to_index(dir);

    let a = Vertex {
        position: [left, top],
        tex_coords: tex[rot],
    };
    let b = Vertex {
        position: [left, bottom],
        tex_coords: tex[(rot + 1) % 4],
    };
    let c = Vertex {
        position: [right, bottom],
        tex_coords: tex[(rot + 2) % 4],
    };
    let d = Vertex {
        position: [right, top],
        tex_coords: tex[(rot + 3) % 4],
    };
    vec![a, b, c, c, d, a]
}
