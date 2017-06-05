extern crate clap;
extern crate image;
extern crate sokoban_backend as sokoban;

use std::path::Path;

use clap::{App, Arg};
use image::{GenericImage, Pixel};

fn main() {
    let matches = App::new("image-to-level")
        .author("Colin Benner <colin@yzhs.de>")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Convert a raster image into a Sokoban level")
        .arg(Arg::with_name("INPUTS")
                 .value_name("FILE")
                 .help("The rastar images to be converted")
                 .required(true)
                 .multiple(true))
        .get_matches();

    for file in matches.values_of("INPUTS").unwrap() {
        println!("\n{}", image_to_level(file));
    }
}

fn image_to_level<P: AsRef<Path>>(path: P) -> String {
    // Parse the image
    let img = image::open(path).unwrap();
    let (width, _) = img.dimensions();

    // Read key
    let empty_color = img.get_pixel(0, 0).to_rgba();
    let wall_color = img.get_pixel(1, 0).to_rgba();
    let floor_color = img.get_pixel(2, 0).to_rgba();
    let worker_color = img.get_pixel(3, 0).to_rgba();
    let crate_on_goal_color = img.get_pixel(4, 0).to_rgba();
    let crate_color = img.get_pixel(5, 0).to_rgba();
    let goal_color = img.get_pixel(6, 0).to_rgba();
    let worker_on_goal_color = img.get_pixel(7, 0).to_rgba();

    // Generate result
    let mut result = "".to_owned();
    let mut tmp = "".to_owned();

    for (x, y, pixel) in img.pixels().skip(width as usize) {
        tmp.push(if pixel == empty_color || pixel == floor_color {
                     ' '
                 } else if pixel == wall_color {
            '#'
        } else if pixel == goal_color {
            '.'
        } else if pixel == crate_on_goal_color {
            '*'
        } else if pixel == crate_color {
            '$'
        } else if pixel == worker_color {
            '@'
        } else if pixel == worker_on_goal_color {
            '+'
        } else {
            panic!("Invalid pixel at ({},{})", x, y)
        });

        if x == width - 1 {
            result.push_str(tmp.trim_right());
            result.push('\n');
            tmp.clear();
        }
    }

    result
}
