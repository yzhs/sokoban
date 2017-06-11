
use glium;
use glium::texture::Texture2d;
use glium::backend::Facade;
use image;

use backend::{ASSETS, Position};


pub struct Textures {
    pub combined: Texture2d,
    pub wall: Texture2d,
    pub wall_left: Texture2d,
    pub wall_right: Texture2d,
    pub wall_both: Texture2d,
    pub floor: Texture2d,
    pub goal: Texture2d,
}

impl Textures {
    /// Load all textures.
    pub fn new(factory: &Facade) -> Self {
        let combined = load(factory, "combined");
        let wall = load(factory, "wall");
        let wall_left = load(factory, "wall_left");
        let wall_right = load(factory, "wall_right");
        let wall_both = load(factory, "wall_both");
        let floor = load(factory, "floor");
        let goal = load(factory, "goal");

        Textures {
            combined,
            wall,
            wall_left,
            wall_right,
            wall_both,
            floor,
            goal,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

/// Pass through coordinates and texture coordinates.
pub const VERTEX_SHADER: &str = r#"
#version 140

in vec2 position;
in vec2 tex_coords;
out vec2 v_tex_coords;

void main() {
    v_tex_coords = tex_coords;
    gl_Position = vec4(position, 0.0, 1.0);
}
"#;


/// Render texture on triangles.
pub const FRAGMENT_SHADER: &str = r#"
#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;

void main() {
    color = texture(tex, v_tex_coords);
}
"#;

#[derive(Clone, Copy, Debug)]
pub enum TileKind {
    Empty,
    Wall,
    Floor,
    Goal,
    Crate,
    Worker,
}

fn lrtp_to_vertices(left: f32, right: f32, top: f32, bottom: f32) -> Vec<Vertex> {
    let a = Vertex {
        position: [left, top],
        tex_coords: [0.0, 0.0],
    };
    let b = Vertex {
        position: [left, bottom],
        tex_coords: [0.0, 1.0],
    };
    let c = Vertex {
        position: [right, top],
        tex_coords: [1.0, 0.0],
    };
    let d = Vertex {
        position: [right, bottom],
        tex_coords: [1.0, 1.0],
    };
    vec![a, b, c, c, b, d]
}


/// Create a bunch of vertices for rendering a textured square.
pub fn create_quad_vertices(pos: Position,
                            columns: u32,
                            rows: u32,
                            aspect_ratio: f32)
                            -> Vec<Vertex> {
    let left = 2.0 * pos.x as f32 / columns as f32 - 1.0;
    let right = left + 2.0 / columns as f32;
    let bottom = -2.0 * pos.y as f32 / rows as f32 + 1.0;
    let top = bottom - 2.0 / rows as f32;

    if aspect_ratio < 1.0 {
        lrtp_to_vertices(left, right, top * aspect_ratio, bottom * aspect_ratio)
    } else {
        lrtp_to_vertices(left / aspect_ratio, right / aspect_ratio, top, bottom)
    }
}


/// Create a rectangle covering the entire viewport.
pub fn create_full_screen_quad() -> Vec<Vertex> {
    let left = -1.0;
    let right = 1.0;
    let top = -1.0;
    let bottom = 1.0;
    lrtp_to_vertices(left, right, top, bottom)
}

/// Create a centered rectangle with the right size to display the static parts of a level with
/// the correct aspect ratio.
pub fn create_background_quad(window_aspect_ratio: f32,
                              columns: usize,
                              rows: usize)
                              -> Vec<Vertex> {
    let aspect_ratio = columns as f32 / rows as f32 * window_aspect_ratio;
    if aspect_ratio < 1.0 {
        lrtp_to_vertices(-aspect_ratio, aspect_ratio, -1.0, 1.0)
    } else {
        lrtp_to_vertices(-1.0, 1.0, -1.0 / aspect_ratio, 1.0 / aspect_ratio)
    }
}

/// Load an image from the assets directory and turn it into a `Texture2d`.
pub fn load(display: &Facade, name: &str) -> Texture2d {
    let mut path = ASSETS.join("images");
    path.push(name);
    path.set_extension("png");
    let image = image::open(path).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(),
                                                                   image_dimensions);
    Texture2d::new(display, image).unwrap()
}
