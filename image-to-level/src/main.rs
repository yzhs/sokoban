extern crate clap;
extern crate image;
extern crate sokoban_backend as sokoban;

use std::fs;
use std::io::{self, Read, Write};
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
        .arg(Arg::with_name("reverse")
                 .help("Turn a level into a raster image")
                 .short("r")
                 .long("reverse"))
        .get_matches();

    if matches.is_present("reverse") {
        for name in matches.values_of("INPUTS").unwrap() {
            write_image_directory(name).unwrap();
        }
    } else {
        for dir in matches.values_of("INPUTS").unwrap() {
            write_collection(dir).unwrap();
        }
    }
}

/// Given the path to a directory containing any number of images and a text file containing the
/// title, create a collection of Sokoban levels in the usual ASCII format.
fn write_collection<P: AsRef<Path>>(dir: P) -> io::Result<()> {
    let mut collection = "".to_string();

    for file in fs::read_dir(&dir)? {
        let path = file?.path();
        if let Some(ext) = path.extension() {
            if ext == std::ffi::OsStr::new("txt") {
                let mut tmp = "".to_string();
                fs::File::open(&path).unwrap().read_to_string(&mut tmp)?;
                collection.push_str(&tmp);
            } else {
                collection.push_str(&image_to_level(&path));
            }
        }
        collection.push('\n');
    }

    let mut output = dir.as_ref().to_path_buf();
    output.set_extension("lvl");
    let mut output_file = fs::File::create(output)?;
    write!(output_file, "{}", collection)
}

/// Read a collection in the Sokoban assets directory and create a directory containing one image
/// for each level of that collection.
fn write_image_directory<P: AsRef<Path>>(name: P) -> io::Result<()> {
    let collection = sokoban::Collection::load(name.as_ref().to_str().unwrap()).unwrap();
    let mut path = name.as_ref().to_path_buf();
    path.set_extension("");

    fs::create_dir(&path).unwrap_or(());
    write!(fs::File::create(path.join("0000_title.txt")).unwrap(),
           "{}",
           collection.name)?;

    for (i, level) in collection.levels().iter().enumerate() {
        level_to_image(path.join(format!("{:04}_level.png", i + 1)), level)?;
    }

    Ok(())
}

/// Generate the ASCII representation of a level given an image.
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

/// Generate an image representation of a given level.
fn level_to_image<P: AsRef<Path>>(target: P, level: &sokoban::Level) -> std::io::Result<()> {
    use image::{Rgb, ImageBuffer};

    const EMPTY_COLOR: Rgb<u8> = Rgb { data: [0, 0, 0] };
    const WALL_COLOR: Rgb<u8> = Rgb { data: [255, 0, 0] };
    const FLOOR_COLOR: Rgb<u8> = Rgb { data: [160, 160, 160] };
    const WORKER_COLOR: Rgb<u8> = Rgb { data: [0, 255, 33] };
    const CRATE_ON_GOAL_COLOR: Rgb<u8> = Rgb { data: [0, 38, 255] };
    const CRATE_COLOR: Rgb<u8> = Rgb { data: [0, 255, 255] };
    const GOAL_COLOR: Rgb<u8> = Rgb { data: [64, 64, 64] };
    const WORKER_ON_GOAL_COLOR: Rgb<u8> = Rgb { data: [255, 216, 0] };

    let width = level.columns() as u32;
    let height = level.rows() as u32 + 1;
    let mut img = ImageBuffer::new(width, height);

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
        img.put_pixel(pos.x as u32, pos.y as u32 + 1, pixel);
    }

    img.save(target)
}
