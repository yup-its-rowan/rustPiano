RustPiano is a working digital piano in Rust that takes in MIDI inputs, then uses a digital synth and soundfont to generate and then audibly play low-latency waveforms!

To run, go to /rustPiano and type 'cargo run'

To build, go to /rustPiano and type 'cargo build --release' for the exe

Inside RustPiano is a folder called /rustPiano/flutter_guis, which holds a flutter app that I use to generate popups.
Popups are just an example for any exe that I can execute through note patterns inside the rust program.
These flutter apps have their exes extracted to /rustPiano/src/popupExe, and can only be run with their surrounding assets.
Inside main.rs is a section that calls these exes. I was unable to bundle the entire directory inside the created rust.exe, which ended up saving memory space when I run the rust exes in my Java traybuddy project. However as a result, a computer that runs this program needs to have the files and relevant directories for "C:/IdeaProjects/rustPiano/src/popupExe/POPUPDIR/flutter_gui.exe"

have fun!