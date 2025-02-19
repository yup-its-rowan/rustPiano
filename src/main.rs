use midir::MidiInput;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::{collections::HashMap, io::Cursor, process::Command, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}};
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

    fn get_value(&self) -> i32 {
        self.value
    }

    fn empty_rulemap(&self) -> bool {
        self.rulemap.is_empty()
    }
}

fn main() -> Result<(), Box<dyn Error>> {

    const LOUD: bool = false;
    const PATTERN: bool = true;

    // initialize note patterns
    let note_patterns = vec![
        vec![96, 95, 96, 95, -2],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, 64, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, 64, 64, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, 64, 64, 62, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, 64, 64, 62, 62, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, 64, 64, 62, 62, 64, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, 64, 64, 62, 62, 64, 62, -3],
        vec![64, 64, 64, 64, 64, 64, 64, 67, 60, 62, 64, 65, 65, 65, 65, 65, 64, 64, 64, 64, 62, 62, 64, 62, 67, -3],
    ];

    let nodes: Arc<Mutex<Vec<Arc<Mutex<Node>>>>> = Arc::new(Mutex::new(Vec::new()));
    let root = Arc::new(Mutex::new(Node::new(-1)));
    

    // Create nodes for each pattern 
    for pattern in note_patterns {
        let mut current_node = root.clone();
        let pattern_len = pattern.len()-1;
        for i in 0..pattern_len {
            let note = pattern[i];
            let next_node = {  // New scope to ensure lock is dropped
                let mut current_node_lock = current_node.lock().unwrap();
                
                if current_node_lock.get_rule(note).is_none() {
                    let next_node = Node::new(pattern[pattern_len]);
                    current_node_lock.add_rule(note, Arc::new(Mutex::new(next_node)));
                }
                
                // Get the next node while we still have the lock
                current_node_lock.get_rule(note).unwrap().clone()
            }; // Lock is dropped here
            
            // Now we can safely assign to current_node
            current_node = next_node;
        }
        current_node.lock().unwrap().add_rule(-1, Arc::new(Mutex::new(Node::new(pattern[pattern_len]))));
    }

    

    // Initialize MIDI input
    let midi_in = MidiInput::new("midi-synthesizer")?;
    let ports = midi_in.ports();
    let midi_port = match ports.get(0) {
        Some(port) => port,
        None => return Err("No MIDI input ports available".into()),
    };

    // Load SoundFont and initialize synthesizer
    let mut sf2 = Box::new(Cursor::new(include_bytes!("piano.sf2")));
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
    if LOUD {
        stream.play()?;
    }

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
                        if custom_velocity > 127 {
                            custom_velocity = 127;
                        }  
                        synth.note_on(0, message[1] as i32, custom_velocity);
                        if PATTERN {
                            interpret_note(Arc::clone(&nodes), Arc::clone(&root), message[1] as i32);
                        }
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

    // Main thread waits for shutdown signal
    while synth_state.running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }

    // Allow time for final cleanup
    thread::sleep(Duration::from_millis(100));
    println!("Shutdown complete.");

    Ok(())
}

fn interpret_note(working_nodes: Arc<Mutex<Vec<Arc<Mutex<Node>>>>>, root: Arc<Mutex<Node>>, note: i32) {
    let mut working_nodes_lock = working_nodes.lock().unwrap();
    let mut new_working_nodes: Vec<Arc<Mutex<Node>>> = Vec::new();
    let number_of_nodes = working_nodes_lock.len();
    for i in 0..number_of_nodes {
        let node_locked = working_nodes_lock[i].lock().unwrap();
        if node_locked.get_rule(note).is_some() {
            if node_locked.get_rule(note).unwrap().lock().unwrap().get_rule(-1).is_some() {
                successful_pattern(node_locked.get_rule(note).unwrap().lock().unwrap().get_value());
            }
            new_working_nodes.push(node_locked.get_rule(note).unwrap().clone());
        }
    }
    let root_locked = root.lock().unwrap();
    if root_locked.get_rule(note).is_some() {
        if root_locked.get_rule(note).unwrap().lock().unwrap().get_rule(-1).is_some() {
            successful_pattern(root_locked.get_rule(note).unwrap().lock().unwrap().get_value());
        }
        new_working_nodes.push(root_locked.get_rule(note).unwrap().clone());
    }
    *working_nodes_lock = new_working_nodes;
}

fn successful_pattern(note: i32) {
    //println!("Note: {}", note);
    run_program(note);
}

//const FREDDY: &[u8] = include_bytes!("freddy.png");
//const SNOOPY: &[u8] = include_bytes!("snoopyChristmas.gif");

fn run_program(note: i32) {
    if note == -2 {
        let _ = {
            Command::new("powershell")
                .args(&["C:/IdeaProjects/rustPiano/src/popupExe/freddy/flutter_guis.exe"])
                .spawn()
        };
    } else if note == -3 {
        let _ = {
            Command::new("powershell")
                .args(&["C:/IdeaProjects/rustPiano/src/popupExe/snoopy/flutter_guis.exe"])
                .spawn()
        };
    } else {
        let _ = {
            Command::new("powershell")
                .args(&["C:/IdeaProjects/rustPiano/src/popupExe/freddy/flutter_guis.exe"])
                .spawn()
        };
    }
}
    