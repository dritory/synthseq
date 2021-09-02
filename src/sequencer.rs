use crate::config;
use crate::engine::Engine;
use crate::note::{NoteEvent, NoteModule, NONE_NOTE};
use synthesizer_io_core::graph::Message;

const NONE_NOTES: [NoteEvent; config::VOICE_COUNT] = [NONE_NOTE; config::VOICE_COUNT];

type Notes = [NoteEvent; config::VOICE_COUNT];

pub struct Sequencer {
    channel: usize,
    bpm: f32,
    steps: [Notes; config::MAX_STEPS],
    current_step: usize,
    scheduled_notes: Notes,
    last_played_notes: Notes,
    step_size: usize,
    sequence_length: usize,
}

impl Sequencer {
    pub fn new(channel: usize, bpm: f32, sequence_length: usize) -> Sequencer {
        Sequencer {
            channel: channel,
            bpm: bpm,
            steps: [NONE_NOTES; config::MAX_STEPS],
            current_step: 0,
            scheduled_notes: NONE_NOTES,
            last_played_notes: NONE_NOTES,
            step_size: 1,
            sequence_length: sequence_length,
        }
    }

    pub fn update_notes(&mut self, note_module: &NoteModule) {
        let voices = note_module.get_voices(0);
        let mut notes = &mut self.scheduled_notes;
        for i in 0..voices.len() {
            if voices[i].note.is_some() {
                notes[i].note = voices[i].note.unwrap();
                notes[i].down = true;
                notes[i].velocity = voices[i].velocity;
                notes[i].timestamp = voices[i].timestamp;
            }
        }
    }

    pub fn tick(&mut self, engine: &mut Engine, note_module: &mut NoteModule) {

        // Send off-notes
        for note in self.last_played_notes.iter() {
            let mut note = note.clone();
            note.down = false;
            note.velocity = 0.0;
            note_module.note_event(engine, note, self.channel);
        }
        
        self.step();


        // Send current notes
        let notes = &mut self.steps[self.current_step];
        for i in 0..notes.len() {
            note_module.note_event(engine, notes[i].clone(), self.channel);
            self.last_played_notes[i].note = notes[i].note;
        }
    }

    pub fn tock(&mut self, engine: &mut Engine, note_module: &mut NoteModule) {
 
        if self.any_scheduled_notes() {
            let notes = &mut self.steps[self.current_step];
            for i in 0..self.scheduled_notes.len() {
                notes[i] = self.scheduled_notes[i].clone();
                self.scheduled_notes[i].down = false;
            }
        }
    }

    fn step(&mut self) {
        self.current_step = (self.current_step + self.step_size) % self.sequence_length;
    }

    fn get_next_step(&self) -> usize {
        (self.current_step + self.step_size) % self.sequence_length
    }

    pub fn get_bpm(&self) -> f32 {
        self.bpm
    }
    pub fn get_current_steps(&self) -> Notes {
        self.steps[self.current_step].clone()
    }


    fn any_scheduled_notes (&self) -> bool {
        let mut scheduled_notes = false;
        for i in 0..self.scheduled_notes.len() {
            if self.scheduled_notes[i].down {
                scheduled_notes = true;
            }
        }
        scheduled_notes
    }
}
