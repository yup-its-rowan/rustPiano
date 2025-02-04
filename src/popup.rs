use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ImagePopup {
    // State for the popup window
    is_open: Arc<Mutex<bool>>,
    image_path: String,
}

impl ImagePopup {
    /// Create a new ImagePopup instance
    pub fn new(image_path: String) -> Self {
        Self {
            is_open: Arc::new(Mutex::new(false)),
            image_path,
        }
    }

    /// Launch the popup window in a separate thread
    pub fn launch(&self) -> thread::JoinHandle<()> {
        // Clone the arc references to move into the thread
        let is_open_clone = Arc::clone(&self.is_open);
        let image_path_clone = self.image_path.clone();

        thread::spawn(move || {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([400.0, 300.0])
                    .with_decorations(true)
                    .with_resizable(true),
                ..Default::default()
            };

            let _ = eframe::run_native(
                "Image Popup",
                options,
                Box::new(|_cc| -> Box<dyn eframe::App> {
                    Box::new(PopupApp {
                        is_open: is_open_clone,
                        image_path: image_path_clone,
                    })
                }),
            );
        })
    }

    /// Check if the popup is currently open
    pub fn is_open(&self) -> bool {
        *self.is_open.lock().unwrap()
    }

    /// Close the popup window
    pub fn close(&self) {
        *self.is_open.lock().unwrap() = false;
    }
}

// Internal application struct for egui
struct PopupApp {
    is_open: Arc<Mutex<bool>>,
    image_path: String,
}

impl eframe::App for PopupApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Load and display the image
            ui.image(&self.image_path);
        });

        // Update the open state
        *self.is_open.lock().unwrap() = true;
    }
}

pub fn start_popup(note: i32) -> ImagePopup {
    let popup;
    if note == -2 {
        popup = ImagePopup::new("src/freddy.png".to_string());
    } else if note == -3 {
        popup = ImagePopup::new("src/snoopyChristmas.gif".to_string());
    } else {
        popup = ImagePopup::new("src/cheese.cheese".to_string());
    }
    return popup;
}