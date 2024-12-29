#[derive(Debug)]
pub enum SynthMessage {
    NoteOn(i32, i32),  // (note, velocity)
    NoteOff(i32),     // note
    Stop,
}