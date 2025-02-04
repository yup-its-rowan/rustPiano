use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use show_image::{ImageView, ImageInfo, create_window};


pub fn main() -> Result<(), Box<dyn std::error::Error>> {

    let file = File::open(Path::new("src/freddy.png")).unwrap();
    let mut reader = std::io::BufReader::new(file);
    let mut pixel_data: Vec<u8> = Vec::new();
    reader.read_to_end(&mut pixel_data).unwrap();


    let image = ImageView::new(ImageInfo::rgb8(1920, 1080), &pixel_data);

    // Create a window with default options and display the image.
    let window = create_window("image", Default::default())?;
    window.set_image("image-001", image)?;

    Ok(())
}