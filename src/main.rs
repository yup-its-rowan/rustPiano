use rustPiano::synth::RealTimeSynth;
use std::{io::{stdin, stdout, Write}, thread, time::Duration};
use midir::{Ignore, MidiInput};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the synthesizer
    //let synth = RealTimeSynth::new("TimGM6mb.sf2")?;
    let synth = RealTimeSynth::new("src/piano.sf2")?;
    
    // Play middle C (note 60) with velocity 100
    synth.play_note(60, 100);
    synth.play_note(80, 100);
    // Keep note playing for 2 seconds
    thread::sleep(Duration::from_secs(3));
    
    // Stop the note
    synth.stop_note(60);
    
    synth.play_note(70, 100);
    // Let release envelope play out
    thread::sleep(Duration::from_secs(2));
    
    // this is the midi stuff now 
    let mut input = String::new();

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);
    
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
        move |_stamp, message, _| {
            if (message[0] & 0xF0) == 0x90 {
                synth.play_note(
                    message[1] as i32,
                    message[2] as i32,
                );
            } else if (message[0] & 0xF0) == 0x80 {
                synth.stop_note(
                    message[1] as i32,
                );
            }
            //println!("{}: {:?} (len = {})", stamp, message, message.len());
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