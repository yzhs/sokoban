#![feature(try_from)]

extern crate piston;
extern crate piston_window;
extern crate graphics;
extern crate gfx_graphics;
extern crate gfx_core;

use piston_window::*;

mod cell;
mod collection;
mod level;
mod util;

mod texture;

use cell::*;
use collection::*;
use level::*;

use texture::*;


const EMPTY: [f32; 4] = [0.0, 0.0, 0.0, 1.0]; // black
const TILE_SIZE: f64 = 50.0;
const IMAGE_SCALE: f64 = TILE_SIZE / 360.0;

pub struct App {
    current_level_set: Option<Collection>,
}

impl App {
    fn level(&self, n: usize) -> Level {
        self.current_level_set
            .clone()
            .map(|x| x.level(n))
            .unwrap()
    }
}

fn main() {
    let collection = Collection::load("original");
    if collection.is_err() {
        panic!("Failed to load level set: {:?}", collection.unwrap_err());
    }

    let app = App { current_level_set: Some(collection.unwrap()) };

    println!("{}", app.level(2));

    let title = "Sokoban";
    let mut window: PistonWindow =
        WindowSettings::new(title, [640, 480])
            .exit_on_esc(true)
            .build()
            .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    window.set_lazy(true);

    let backgrounds = load_backgrounds(&mut window.factory);
    let foregrounds = load_foregrounds(&mut window.factory);

    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            // Set background
            clear(EMPTY, g);
            let level = &app.level(2);
            let background = &level.background;
            let foreground = &level.foreground;
            for (i, bg) in background.iter().enumerate() {
                if bg == &Background::Empty {
                    continue;
                }

                let x = TILE_SIZE * (i % level.width) as f64;
                let y = TILE_SIZE * (i / level.width) as f64;
                image(&backgrounds[bg],
                      c.transform.trans(x, y).scale(IMAGE_SCALE, IMAGE_SCALE),
                      g);
                // TODO load images instead
            }
            for (i, fg) in foreground.iter().enumerate() {
                if fg == &Foreground::None {
                    continue;
                }

                let x = TILE_SIZE * (i % level.width) as f64;
                let y = TILE_SIZE * (i / level.width) as f64;
                image(&foregrounds[fg],
                      c.transform.trans(x, y).scale(IMAGE_SCALE, IMAGE_SCALE),
                      g);
                // TODO load images instead
            }
        });

        match e.press_args() {
            None => {}
            Some(Button::Keyboard(Key::Left)) => println!("app.move(Direction::Left)"),
            Some(Button::Keyboard(Key::Right)) => println!("app.move(Direction::Right)"),
            Some(Button::Keyboard(Key::Up)) => println!("app.move(Direction::Up)"),
            Some(Button::Keyboard(Key::Down)) => println!("app.move(Direction::Down)"),
            Some(Button::Keyboard(_)) => println!("unkown key"),
            Some(_) => println!("unkown event"),
        };
    }
}
