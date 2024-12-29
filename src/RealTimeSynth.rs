use rustysynth::{SoundFont, SynthesizerSettings, Synthesizer};
use tinyaudio::prelude::*;
use std::fs::File;
use std::io::Read;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

// Define messages for our synthesizer
enum SynthMessage {
    NoteOn(u8, u8),  // (note, velocity)
    NoteOff(u8),     // note
    Stop,
}

pub struct RealTimeSynth {
    sender: Sender<SynthMessage>,
    _device: OutputDevice,
}

impl RealTimeSynth {
    pub fn new(soundfont_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Load the SoundFont file
        let mut sf2_data = Vec::new();
        File::open(soundfont_path)?
            .read_to_end(&mut sf2_data)?;
        
        let sound_font = SoundFont::new(&sf2_data)?;
        let settings = SynthesizerSettings::new(44100);
        let synthesizer = Synthesizer::new(&sound_font, &settings)?;
        let synthesizer = Arc::new(Mutex::new(synthesizer));
        
        let (sender, receiver) = channel();
        
        // Audio parameters
        let sample_rate = 44100;
        let buffer_size = 4096;
        let mut left_buffer = vec![0f32; buffer_size];
        let mut right_buffer = vec![0f32; buffer_size];
        
        let synth_clone = synthesizer.clone();
        let receiver = Arc::new(Mutex::new(receiver));
        let receiver_clone = receiver.clone();
        
        let params = OutputDeviceParameters {
            channels_count: 2,
            sample_rate,
            channel_sample_count: buffer_size,
        };
        
        // Start audio processing
        let device = run_output_device(params, {
            move |data| {
                let mut synth = synth_clone.lock().unwrap();
                
                // Process any pending messages
                if let Ok(receiver) = receiver_clone.try_lock() {
                    while let Ok(msg) = receiver.try_recv() {
                        match msg {
                            SynthMessage::NoteOn(note, velocity) => {
                                synth.note_on(0, note, velocity);
                            }
                            SynthMessage::NoteOff(note) => {
                                synth.note_off(0, note);
                            }
                            SynthMessage::Stop => {
                                // Reset all controllers and notes
                                for i in 0..128 {
                                    synth.note_off(0, i);
                                }
                            }
                        }
                    }
                }
                
                // Render audio
                synth.render(&mut left_buffer, &mut right_buffer);
                
                // Interleave left and right channels
                for i in 0..buffer_size {
                    data[i * 2] = left_buffer[i];
                    data[i * 2 + 1] = right_buffer[i];
                }
            }
        })?;
        
        Ok(RealTimeSynth {
            sender,
            _device: device,
        })
    }
    
    pub fn play_note(&self, note: u8, velocity: u8) {
        let _ = self.sender.send(SynthMessage::NoteOn(note, velocity));
    }
    
    pub fn stop_note(&self, note: u8) {
        let _ = self.sender.send(SynthMessage::NoteOff(note));
    }
    
    pub fn stop_all(&self) {
        let _ = self.sender.send(SynthMessage::Stop);
    }
}

impl Drop for RealTimeSynth {
    fn drop(&mut self) {
        self.stop_all();
    }
}