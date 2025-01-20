use midir::MidiInput;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::{collections::HashMap, fs::File, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}};
use std::error::Error;
use std::time::Duration;
use std::thread;

struct SynthState {
    synthesizer: Mutex<Synthesizer>,
    running: Arc<AtomicBool>,
}

impl SynthState {
    fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

struct Node {
    value: i32,
    rulemap: HashMap<i32, Arc<Mutex<Node>>>,
}

impl Node {
    fn new(value: i32) -> Node {
        Node {
            value,
            rulemap: HashMap::new(),
        }
    }

    fn add_rule(&mut self, rule: i32, node: Arc<Mutex<Node>>) {
        self.rulemap.insert(rule, node);
    }

    fn get_rule(&self, rule: i32) -> Option<&Arc<Mutex<Node>>> {
        self.rulemap.get(&rule)
    }
}

fn main() -> Result<(), Box<dyn Error>> {

    // initialize note patterns
    let note_patterns = vec![
        vec![80, 81, 82],
        vec![60, 60, 61],
    ];

    let nodes: Vec<Node> = Vec::new();
    let root = Arc::new(Mutex::new(Node::new(-1)));
    

    // Create nodes for each pattern 
    for pattern in note_patterns {
        let mut current_node = root.clone();
        for note in pattern {
            let next_node = {  // New scope to ensure lock is dropped
                let mut current_node_lock = current_node.lock().unwrap();
                
                if current_node_lock.get_rule(note).is_none() {
                    let next_node = Node::new(note);
                    current_node_lock.add_rule(note, Arc::new(Mutex::new(next_node)));
                }
                
                // Get the next node while we still have the lock
                current_node_lock.get_rule(note).unwrap().clone()
            }; // Lock is dropped here
            
            // Now we can safely assign to current_node
            current_node = next_node;
        }
    }


    

    // Initialize MIDI input
    let midi_in = MidiInput::new("midi-synthesizer")?;
    let ports = midi_in.ports();
    let midi_port = match ports.get(0) {
        Some(port) => port,
        None => return Err("No MIDI input ports available".into()),
    };

    // Load SoundFont and initialize synthesizer
    let mut sf2 = File::open("src/piano.sf2").unwrap();
    let sound_font = Arc::new(SoundFont::new(&mut sf2).unwrap());
    let settings = SynthesizerSettings::new(44100);
    let synthesizer = Synthesizer::new(&sound_font, &settings)?;
    
    let running = Arc::new(AtomicBool::new(true));
    let synth_state = Arc::new(SynthState {
        synthesizer: Mutex::new(synthesizer),
        running: Arc::clone(&running),
    });

    // Initialize audio output
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or("No output device available")?;
    
    let config = device.default_output_config()?;
    let channels = config.channels() as usize;

    // Create audio stream with minimal buffer size
    let synth_state_audio = Arc::clone(&synth_state);
    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if !synth_state_audio.running.load(Ordering::SeqCst) {
                // Fill buffer with silence when shutting down
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
                return;
            }

            let mut synth = synth_state_audio.synthesizer.lock().unwrap();
            
            let needed_samples = data.len() / channels;
            let mut left_buffer = vec![0f32; needed_samples];
            let mut right_buffer = vec![0f32; needed_samples];
            
            synth.render(&mut left_buffer[..], &mut right_buffer[..]);
            
            for (i, frame) in data.chunks_mut(channels).enumerate() {
                frame[0] = left_buffer[i];
                if channels > 1 {
                    frame[1] = right_buffer[i];
                }
            }
        },
        move |err| eprintln!("Audio stream error: {}", err),
        Some(Duration::from_micros(500))
    )?;

    stream.play()?;

    // Set up MIDI input callback
    let synth_state_midi = Arc::clone(&synth_state);
    let _midi_connection = midi_in.connect(
        midi_port,
        "midi-input",
        move |_timestamp, message, _| {
            if !synth_state_midi.running.load(Ordering::SeqCst) {
                return;
            }
            
            let mut synth = synth_state_midi.synthesizer.lock().unwrap();
            match message[0] & 0xF0 {
                0x90 => { // Note On
                    if message[2] > 0 {
                        let mut custom_velocity = message[2] as i32 * 2.3 as i32;
                        if (custom_velocity > 127) {
                            custom_velocity = 127;
                        }
                        
                        synth.note_on(0, message[1] as i32, custom_velocity);
                    } else {
                        synth.note_off(0, message[1] as i32);
                    }
                },
                0x80 => { // Note Off
                    synth.note_off(0, message[1] as i32);
                },
                _ => {}
            }
        },
        (),
    )?;

    // Create a clone for shutdown function
    let synth_state_shutdown = Arc::clone(&synth_state);
    
    // Spawn a thread to handle shutdown command
    thread::spawn(move || {
        println!("Press 'q' and Enter to quit...");
        loop {
            let mut input = String::new();
            if let Ok(_) = std::io::stdin().read_line(&mut input) {
                if input.trim().to_lowercase() == "q" {
                    println!("Shutting down...");
                    synth_state_shutdown.shutdown();
                    break;
                }
            }
        }
    });

    // Main thread waits for shutdown signal
    while synth_state.running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }

    // Allow time for final cleanup
    thread::sleep(Duration::from_millis(100));
    println!("Shutdown complete.");

    Ok(())
}