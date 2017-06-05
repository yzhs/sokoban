extern crate image;
extern crate sokoban_backend as sokoban;

use image::{GenericImage, Pixel};

fn main() {
    // Parse the image
    let img = image::open("planar_crossover.png").unwrap();
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

    println!("{}", result);
}
