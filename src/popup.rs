use image::{GenericImageView, ImageReader};

use show_image::{ImageView, ImageInfo, create_window};


pub fn main(path: String) -> Result<(), Box<dyn std::error::Error>> {
    let image = ImageReader::open(path)?.decode()?;
    
    let pixel_data: Vec<u8> = image.to_rgb8().into_raw();
    let dimensions = image.dimensions();

    let image = ImageView::new(ImageInfo::rgb8(dimensions.0, dimensions.1), &pixel_data);

    // Create a window with default options and display the image.
    let window = create_window("image", Default::default())?;
    window.set_image("image-001", image)?;

    Ok(())
}