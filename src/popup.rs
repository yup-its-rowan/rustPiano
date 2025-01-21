use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use image::GenericImageView;
use pixels::{Pixels, SurfaceTexture};
use winit::window::{Window, WindowBuilder};

struct ImageWindow {
    window: Window,
    pixels: Pixels,
    image_data: Vec<u8>,
    width: u32,
    height: u32,
}


pub fn show_image_popup(image_path: &str, popup_title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    
    // Load the image
    let image = image::open(image_path)?;
    let (width, height) = image.dimensions();
    
    // Create the window
    let window = WindowBuilder::new()
        .with_title(popup_title)
        .with_inner_size(winit::dpi::LogicalSize::new(width, height))
        .with_decorations(true)   // Set to false if you don't want window borders
        .build(&event_loop)?;

    // Create pixel buffer
    let surface_texture = SurfaceTexture::new(width, height, &window);
    let pixels = Pixels::new(width, height, surface_texture)?;
    
    // Convert image to RGBA
    let image_rgba = image.into_rgba8();
    let image_data = image_rgba.into_raw();
    
    let mut image_window = ImageWindow {
        window,
        pixels,
        image_data,
        width,
        height,
    };
    
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::RedrawRequested(_) => {
                draw_image(&mut image_window);
            }
            Event::MainEventsCleared => {
                image_window.window.request_redraw();
            }
            _ => {}
        }
    });
}

fn draw_image(image_window: &mut ImageWindow) {
    let frame = image_window.pixels.frame_mut();
    
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        let x = (i % image_window.width as usize) * 4;
        let y = (i / image_window.width as usize) * 4;
        let index = y * image_window.width as usize + x;
        
        if index + 3 < image_window.image_data.len() {
            pixel.copy_from_slice(&image_window.image_data[index..index + 4]);
        }
    }
    
    if let Err(err) = image_window.pixels.render() {
        eprintln!("Error rendering pixels: {}", err);
    }
}