use std::rc::Rc;

use glium::Surface;
use glium_text::{self, FontTexture, TextDisplay};

use gui::font::*;

pub struct TextObjectManager {
    font_data: Rc<FontData>,

    text_objects: Vec<TextObject>,
}

impl TextObjectManager {
    pub fn new(font_data: Rc<FontData>) -> Self {
        Self {
            font_data,
            text_objects: vec![],
        }
    }
}

struct TextObject {
    position: [f32; 2],
    scale: f32,
    text_display: TextDisplay<Rc<FontTexture>>,
}

impl TextObject {
    pub fn new(
        font_data: &Rc<FontData>,
        position: [f32; 2],
        scale: f32,
        font_type: FontStyle,
        text: &str,
    ) -> Self {
        let text_display = font_data.create_text_display(font_type, text);
        Self {
            position,
            scale,
            text_display,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text_display.set_text(text);
    }
}

pub type TextObjectHandle = usize;

impl TextObjectManager {
    pub fn create_text_object(
        &mut self,
        position: [f32; 2],
        scale: f32,
        font_type: FontStyle,
        text: &str,
    ) -> TextObjectHandle {
        let handle = self.text_objects.len();
        let text_object = TextObject::new(&self.font_data, position, scale, font_type, text);
        self.text_objects.push(text_object);
        handle
    }

    pub fn set_text(&mut self, handle: TextObjectHandle, text: &str) {
        self.text_objects[handle].set_text(text);
    }

    pub fn draw_text_objects<S: Surface>(&self, target: &mut S, aspect_ratio: f32) {
        for text_object in &self.text_objects {
            self.draw_text_display(
                target,
                &text_object.text_display,
                text_object.scale,
                text_object.position,
                aspect_ratio,
            );
        }
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

        glium_text::draw(text_display, &self.font_data.system, target, matrix, WHITE);
    }
}
