use crate::config;
use crate::engine::Engine;
use crate::note::{NoteEvent, NoteModule, NONE_NOTE};
use synthesizer_io_core::graph::Message;

const NONE_NOTES: Vec<NoteEvent> = vec![];

type Notes = Vec<NoteEvent>;

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

    pub fn update_note(&mut self, note_event : NoteEvent){
        let mut notes = &mut self.scheduled_notes;
        notes.push(note_event);
    }

    pub fn tick(&mut self, engine: &mut Engine, note_module: &mut NoteModule) {

        // Turn off last notes
        for note in self.last_played_notes.iter() {
            let mut note = note.clone();
            note.down = false;
            note.velocity = 0.0;
            note_module.note_event(engine, note, self.channel);
        }
        self.last_played_notes = vec![];


        self.step();
        
        
        let notes = &mut self.steps[self.current_step];
        for note in notes {
            note_module.note_event(engine, note.clone(), self.channel);

            self.last_played_notes.push(note.clone())
        }
    }

    pub fn tock (&mut self, engine: &mut Engine, note_module: &mut NoteModule) {

        if self.scheduled_notes.len() > 0 {
            self.steps[self.current_step] = vec![];
            
            let mut notes = &mut self.steps[self.current_step];

            for i in 0..self.scheduled_notes.len() {
                notes.push(self.scheduled_notes[i].clone());
                
            }
            self.scheduled_notes = vec![];
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
}
