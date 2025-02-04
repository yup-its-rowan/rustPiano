use image::{GenericImageView, ImageReader};

use std::{io::Cursor, sync::Arc};
use image::{ImageBuffer, Rgba};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder, WindowLevel},
    platform::windows::EventLoopBuilderExtWindows,
};

pub fn main(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // Create event loop
    let event_loop = EventLoopBuilder::new()
        .with_any_thread(true)
        .build()?;

    let image = ImageReader::new(Cursor::new(data)).with_guessed_format()?.decode()?;
    let pixel_data: Vec<u8> = image.to_rgb8().into_raw();
    // Assume we have image dimensions
    let width = image.dimensions().0;  // Replace with your image width
    let height = image.dimensions().1; // Replace with your image height

    // Create window
    let window = WindowBuilder::new()
        .with_title("Image Display")
        .with_inner_size(LogicalSize::new(width, height))
        .with_window_level(WindowLevel::AlwaysOnTop) // This makes it stay on top
        .with_resizable(false)
        .build(&event_loop)?;

    // Create pixels buffer
    let surface_texture = SurfaceTexture::new(width, height, &window);
    let mut pixels = Pixels::new(width, height, surface_texture)?;

    // Convert your u8 buffer to RGBA
    // Assuming your_buffer is your image data
    //let image_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixel_data).unwrap();
    
    // For demonstration, let's create a simple gradient
    let image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([
            (x % 255) as u8,
            (y % 255) as u8,
            ((x + y) % 255) as u8,
            255,
        ])
    });

    // Main event loop
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();

            }
            Event::WindowEvent { 
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // Copy image data to frame
                let frame = pixels.frame_mut();
                frame.copy_from_slice(&image_buffer.as_raw());
                
                // Render the frame
                if let Err(e) = pixels.render() {
                    eprintln!("Render error: {}", e);
                    elwt.exit();
                }
            }
            _ => ()
        }
    })?;

    Ok(())
}

