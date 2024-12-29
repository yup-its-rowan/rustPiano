use std::error::Error;
use std::fs::File;
use std::io::{stdin, stdout, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use midir::{Ignore, MidiInput};
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
use tinyaudio::{OutputDeviceParameters, prelude::*};
use itertools::Itertools;

fn main() -> Result<(), Box<dyn std::error::Error>> { 

    // Load the SoundFont.
    let mut sf2 = File::open("src/piano.sf2").unwrap();
    let sound_font = Arc::new(SoundFont::new(&mut sf2).unwrap());
    
    // Configure synthesizer settings
    let settings = SynthesizerSettings::new(44100);
    let mut synthesizer = Synthesizer::new(&sound_font, &settings)?;

    // Wrap synthesizer in Arc<Mutex> for thread-safe access
    let synthesizer = Arc::new(Mutex::new(synthesizer));
    let synthesizer_clone = synthesizer.clone();
    
    // MIDI note number for middle C (60)
    let note = 60;
    let velocity = 100; // Note velocity (0-127)
    let channel = 0;    // MIDI channel
    
    // Audio parameters
    let sample_rate = 44100;
    let buffer_size = 4096;
    let mut left_buffer = vec![0f32; buffer_size];
    let mut right_buffer = vec![0f32; buffer_size];

    // Flag to track if we've started the note
    let note_started = Arc::new(AtomicBool::new(false));
    let note_started_clone = note_started.clone();

    let params = OutputDeviceParameters {
        channels_count: 2,
        sample_rate,
        channel_sample_count: buffer_size,
    };

    let device = run_output_device(params, {
        move |data| {
            let mut synth = synthesizer_clone.lock().unwrap();
            
            // Only trigger the note once
            if !note_started_clone.load(Ordering::Relaxed) {
                synth.note_on(channel, note, velocity);
                note_started_clone.store(true, Ordering::Relaxed);
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

    // Keep the program running for 5 seconds
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // Release the note
    {
        let mut synth = synthesizer.lock().unwrap();
        synth.note_off(channel, note);
    }
    
    // Let the release envelope play out
    std::thread::sleep(std::time::Duration::from_secs(2));

    drop(device);

    Ok(())
    /*
    match run() {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    }
    */
}

fn run() -> Result<(), Box<dyn Error>> {
    
    let mut input = String::new();

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    
    // Load the SoundFont.
    let mut sf2 = File::open("src/piano.sf2").unwrap();
    let sound_font = Arc::new(SoundFont::new(&mut sf2).unwrap());

    // Create the synthesizer.
    let settings = SynthesizerSettings::new(44100);
    let mut synthesizer = Synthesizer::new(&sound_font, &settings).unwrap();

    // Play some notes (middle C, E, G).
    synthesizer.note_on(0, 60, 100);
    synthesizer.note_on(0, 64, 100);
    synthesizer.note_on(0, 67, 100);

    // The output buffer (3 seconds).
    let sample_count = (3 * settings.sample_rate) as usize;
    let mut left: Vec<f32> = vec![0_f32; sample_count];
    let mut right: Vec<f32> = vec![0_f32; sample_count];

    // Render the waveform.
    synthesizer.render(&mut left[..], &mut right[..]);
    
    // Get an input port (read from console if multiple are available)
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => return Err("no input port found".into()),
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            in_ports
                .get(input.trim().parse::<usize>()?)
                .ok_or("invalid input port selected")?
        }
    };

    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(in_port)?;

    // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
    let _conn_in = midi_in.connect(
        in_port,
        "midir-read-input",
        move |stamp, message, _| {
            println!("{}: {:?} (len = {})", stamp, message, message.len());
        },
        (),
    )?;

    println!(
        "Connection open, reading input from '{}' (press enter to exit) ...",
        in_port_name
    );

    input.clear();
    stdin().read_line(&mut input)?; // wait for next enter key press

    println!("Closing connection");
    Ok(())
}