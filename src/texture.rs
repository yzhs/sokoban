
use glium;
use glium::backend::Facade;
use image;

use backend::{self, ASSETS};


#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);


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


pub const FRAGMENT_SHADER: &str = r#"
#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;

void main() {
    color = texture(tex, v_tex_coords);
}
"#;


fn lrtp_to_vertices(left: f32, right: f32, top: f32, bottom: f32) -> Vec<Vertex> {
    vec![Vertex {
             position: [left, top],
             tex_coords: [0.0, 0.0],
         },
         Vertex {
             position: [left, bottom],
             tex_coords: [0.0, 1.0],
         },
         Vertex {
             position: [right, top],
             tex_coords: [1.0, 0.0],
         },
         Vertex {
             position: [right, bottom],
             tex_coords: [1.0, 1.0],
         }]
}

/// Create a bunch of vertices for rendering a textured square.
pub fn create_quad_vertices(pos: backend::Position, columns: u32, rows: u32) -> Vec<Vertex> {
    let left = 2.0 * pos.x as f32 / columns as f32 - 1.0;
    let right = left + 2.0 / columns as f32;
    let bottom = -2.0 * pos.y as f32 / rows as f32 + 1.0;
    let top = bottom - 2.0 / rows as f32;
    lrtp_to_vertices(left, right, top, bottom)
}

pub fn create_full_screen_quad() -> Vec<Vertex> {
    let left = -1.0;
    let right = 1.0;
    let top = -1.0;
    let bottom = 1.0;
    lrtp_to_vertices(left, right, top, bottom)
}

/// Load an image from the assets directory and turn it into a `Texture2d`.
pub fn load(display: &Facade, name: &str) -> glium::texture::Texture2d {
    let mut path = ASSETS.join("images");
    path.push(name);
    path.set_extension("png");
    let image = image::open(path).unwrap().to_rgba();
    let image_dimensions = image.dimensions();
    let image = glium::texture::RawImage2d::from_raw_rgba_reversed(image.into_raw(),
                                                                   image_dimensions);
    glium::texture::Texture2d::new(display, image).unwrap()
}
