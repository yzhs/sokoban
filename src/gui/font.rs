use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use glium::backend::glutin::Display;
use glium::Surface;
use glium_text::{draw, FontTexture, TextDisplay, TextSystem};

pub const WHITE: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 1.0);

#[derive(Clone, Copy)]
pub enum FontStyle {
    Heading,
    Text,
    Mono,
}

/// Collection of glyph textures.
pub struct FontData {
    pub system: TextSystem,
    heading_font: Rc<FontTexture>,
    text_font: Rc<FontTexture>,
    mono_font: Rc<FontTexture>,
}

impl FontData {
    /// Load font from disk and create a glyph texture at two different font sizes.
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(
        display: &Display,
        font_path: P,
        mono_path: Q,
    ) -> Self {
        let system = TextSystem::new(display);
        let text_font =
            Rc::new(FontTexture::new(display, File::open(&font_path).unwrap(), 32).unwrap());
        let heading_font =
            Rc::new(FontTexture::new(display, File::open(&font_path).unwrap(), 64).unwrap());
        let mono_font =
            Rc::new(FontTexture::new(display, File::open(&mono_path).unwrap(), 32).unwrap());

        FontData {
            system,
            heading_font,
            text_font,
            mono_font,
        }
    }

    fn font_type_to_font(&self, font_type: FontStyle) -> Rc<FontTexture> {
        match font_type {
            FontStyle::Heading => self.heading_font.clone(),
            FontStyle::Text => self.text_font.clone(),
            FontStyle::Mono => self.mono_font.clone(),
        }
    }

    pub fn create_text_display(
        &self,
        font_type: FontStyle,
        text: &str,
    ) -> TextDisplay<Rc<FontTexture>> {
        let font = self.font_type_to_font(font_type);
        TextDisplay::new(&self.system, font, text)
    }

    pub fn draw_text_display<S: Surface>(
        &self,
        target: &mut S,
        text_display: &TextDisplay<Rc<FontTexture>>,
        scale: f32,
        position: [f32; 2],
        aspect_ratio: f32,
    ) {
        let x = position[0] * scale * text_display.get_width();
        let y = position[1];
        let matrix = [
            [scale, 0.0, 0.0, 0.0],
            [0.0, scale / aspect_ratio, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, 0.0, 1.0_f32],
        ];

        draw(text_display, &self.system, target, matrix, WHITE);
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
        let text_display = self.create_text_display(font_type, text);
        self.draw_text_display(target, &text_display, scale, offset, aspect_ratio);
    }
}
