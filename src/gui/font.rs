use std::fs::File;
use std::path::Path;

use glium::backend::glutin::Display;
use glium::Surface;
use glium_text::{draw, FontTexture, TextDisplay, TextSystem};

const WHITE: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 1.0);

#[derive(Clone, Copy)]
pub enum FontStyle {
    Heading,
    Text,
    Mono,
}

/// Collection of glyph textures.
pub struct FontData {
    system: TextSystem,
    heading_font: FontTexture,
    text_font: FontTexture,
    mono_font: FontTexture,
}

impl FontData {
    /// Load font from disk and create a glyph texture at two different font sizes.
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(
        display: &Display,
        font_path: P,
        mono_path: Q,
    ) -> Self {
        let system = TextSystem::new(display);
        let text_font = FontTexture::new(display, File::open(&font_path).unwrap(), 16).unwrap();
        let heading_font = FontTexture::new(display, File::open(&font_path).unwrap(), 48).unwrap();
        let mono_font = FontTexture::new(display, File::open(&mono_path).unwrap(), 16).unwrap();

        FontData {
            system,
            heading_font,
            text_font,
            mono_font,
        }
    }

    /// Draw text in the specified font. Scale by `scale` and move to a given position. Correct
    /// for aspect ratio.
    pub fn draw<S: Surface>(
        &self,
        target: &mut S,
        text: &str,
        font_type: FontStyle,
        scale: f32,
        offset: [f32; 2],
        aspect_ratio: f32,
    ) {
        let font = match font_type {
            FontStyle::Heading => &self.heading_font,
            FontStyle::Text => &self.text_font,
            FontStyle::Mono => &self.mono_font,
        };
        let text_display = TextDisplay::new(&self.system, font, text);
        let x = offset[0] * scale * text_display.get_width();
        let y = offset[1];
        let matrix = [
            [scale, 0.0, 0.0, 0.0],
            [0.0, scale / aspect_ratio, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, 0.0, 1.0_f32],
        ];

        draw(&text_display, &self.system, target, matrix, WHITE);
    }
}
