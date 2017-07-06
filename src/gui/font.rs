use std::fs::File;
use std::path::Path;

use glium::Frame;
use glium::backend::glutin_backend::GlutinFacade;
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};


const WHITE: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 1.0);

pub enum Font {
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
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(display: &GlutinFacade,
                                               font_path: P,
                                               mono_path: Q)
                                               -> Self {
        let system = TextSystem::new(display);
        let text_font = FontTexture::new(display,
                                         File::open(&font_path).unwrap(),
                                         32,
                                         FontTexture::ascii_character_list())
                .unwrap();
        let heading_font = FontTexture::new(display,
                                            File::open(&font_path).unwrap(),
                                            64,
                                            FontTexture::ascii_character_list())
                .unwrap();

        let mono_font = FontTexture::new(display,
                                         File::open(&mono_path).unwrap(),
                                         32,
                                         FontTexture::ascii_character_list())
                .unwrap();

        FontData {
            system,
            heading_font,
            text_font,
            mono_font,
        }
    }

    pub fn draw(&self,
                target: &mut Frame,
                text: &str,
                font_type: Font,
                scale: f32,
                offset: [f32; 2],
                aspect_ratio: f32) {

        let font = match font_type {
            Font::Heading => &self.heading_font,
            Font::Text => &self.text_font,
            Font::Mono => &self.mono_font,
        };
        let text_display = TextDisplay::new(&self.system, font, text);
        let matrix = [[scale, 0.0, 0.0, 0.0],
                      [0.0, scale / aspect_ratio, 0.0, 0.0],
                      [0.0, 0.0, 1.0, 0.0],
                      [offset[0] * scale * text_display.get_width(),
                       offset[1],
                       0.0,
                       1.0_f32]];

        let _ = ::glium_text_rusttype::draw(&text_display, &self.system, target, matrix, WHITE);
    }
}
