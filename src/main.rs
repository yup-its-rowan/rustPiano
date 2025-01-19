use midir::MidiInput;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::fs::File;

struct SynthState {
    synthesizer: Mutex<Synthesizer>,
}

fn main() -> Result<(), Box<dyn Error>> {
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
    
    let synth_state = Arc::new(SynthState {
        synthesizer: Mutex::new(synthesizer),
    });

    // Initialize audio output
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or("No output device available")?;
    
    let config = device.default_output_config()?;
    //let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    // Create audio stream with minimal buffer size
    let synth_state_audio = Arc::clone(&synth_state);
    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut synth = synth_state_audio.synthesizer.lock().unwrap();
            
            // Calculate how many samples we need
            let needed_samples = data.len() / channels;
            let mut left_buffer = vec![0f32; needed_samples];
            let mut right_buffer = vec![0f32; needed_samples];
            
            // Render audio
            synth.render(&mut left_buffer[..], &mut right_buffer[..]);
            
            // Interleave the channels
            for (i, frame) in data.chunks_mut(channels).enumerate() {
                frame[0] = left_buffer[i];
                if channels > 1 {
                    frame[1] = right_buffer[i];
                }
            }
        },
        move |err| eprintln!("Audio stream error: {}", err),
        Some(std::time::Duration::from_millis(1)), // Low latency buffer
    )?;

    stream.play()?;

    // Set up MIDI input callback
    let synth_state_midi = Arc::clone(&synth_state);
    let _midi_connection = midi_in.connect(
        midi_port,
        "midi-input",
        move |_timestamp, message, _| {
            let mut synth = synth_state_midi.synthesizer.lock().unwrap();
            match message[0] & 0xF0 {
                0x90 => { // Note On
                    if message[2] > 0 {
                        synth.note_on(0, message[1] as i32, message[2] as i32);
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

    println!("Synthesizer running. Press Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(())
}