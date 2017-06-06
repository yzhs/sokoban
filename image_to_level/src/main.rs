extern crate clap;
extern crate image;
extern crate sokoban_backend as sokoban;

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

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
        .arg(Arg::with_name("reverse")
                 .help("Turn a level into a raster image")
                 .short("r")
                 .long("reverse"))
        .get_matches();

    for dir in matches.values_of("INPUTS").unwrap() {
        let collection = directory_to_collection(dir).unwrap();
        let mut output = PathBuf::new();
        output.push(dir);
        output.set_extension("lvl");
        let mut output_file = File::create(output).unwrap();
        write!(output_file, "{}", collection).unwrap();
    }
}

fn directory_to_collection<P: AsRef<Path>>(dir: P) -> io::Result<String> {
    use std::fs::read_dir;

    let mut result = "".to_string();

    let dir = dir.as_ref();
    for file in read_dir(dir)? {
        let path = file?.path();
        if let Some(ref ext) = path.extension() {
            if ext == &std::ffi::OsStr::new("txt") {
                let mut tmp = "".to_string();
                File::open(&path).unwrap().read_to_string(&mut tmp)?;
                result.push_str(&tmp);
            } else {
                result.push_str(&image_to_level(&path));
            }
        }
        result.push('\n');
    }

    Ok(result)
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

fn level_to_image<P: AsRef<Path>>(target: P, level: sokoban::Level) -> std::io::Result<()> {
    use image::{Rgb, ImageBuffer};
    let mut img = ImageBuffer::new(level.columns() as u32 + 1, level.rows() as u32);

    const EMPTY_COLOR: Rgb<u8> = Rgb { data: [0, 0, 0] };
    const WALL_COLOR: Rgb<u8> = Rgb { data: [255, 0, 0] };
    const FLOOR_COLOR: Rgb<u8> = Rgb { data: [160, 160, 160] };
    const WORKER_COLOR: Rgb<u8> = Rgb { data: [0, 255, 33] };
    const CRATE_ON_GOAL_COLOR: Rgb<u8> = Rgb { data: [0, 38, 255] };
    const CRATE_COLOR: Rgb<u8> = Rgb { data: [0, 255, 255] };
    const GOAL_COLOR: Rgb<u8> = Rgb { data: [64, 64, 64] };
    const WORKER_ON_GOAL_COLOR: Rgb<u8> = Rgb { data: [255, 216, 0] };


    // Write key into first row
    for i in 0..level.columns() {
        img.put_pixel(i as u32, 0, EMPTY_COLOR);
    }
    img.put_pixel(0, 0, EMPTY_COLOR);
    img.put_pixel(1, 0, WALL_COLOR);
    img.put_pixel(2, 0, FLOOR_COLOR);
    img.put_pixel(3, 0, WORKER_COLOR);
    img.put_pixel(4, 0, CRATE_ON_GOAL_COLOR);
    img.put_pixel(5, 0, CRATE_COLOR);
    img.put_pixel(6, 0, GOAL_COLOR);
    img.put_pixel(7, 0, WORKER_ON_GOAL_COLOR);

    // Write level into remaining rows
    for (i, &bg) in level.background.iter().enumerate() {
        use sokoban::Background;
        let pos = level.position(i);
        let pixel = match bg {
            Background::Empty => EMPTY_COLOR,
            Background::Wall => WALL_COLOR,
            Background::Floor => {
                if level.crates.contains_key(&pos) {
                    CRATE_COLOR
                } else if level.worker_position == pos {
                    WORKER_COLOR
                } else {
                    FLOOR_COLOR
                }
            }
            Background::Goal => {
                if level.crates.contains_key(&pos) {
                    CRATE_ON_GOAL_COLOR
                } else if level.worker_position == pos {
                    WORKER_ON_GOAL_COLOR
                } else {
                    GOAL_COLOR
                }
            }
        };
        img.put_pixel(pos.x as u32 + 1, pos.y as u32, pixel);
    }

    img.save(target)
}
