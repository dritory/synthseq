use crate::engine::{ControlMap, Engine};
use synthesizer_io_core::graph::{IntoBoxedSlice, Message, Node, Note, SetParam};


pub struct NoteModule {
    voices: Vec<Voice>
}


impl NoteModule {
    pub fn new(voice_count : usize) -> NoteModule {
        NoteModule {
            voices: vec![Voice{note: None, timestamp: 0}; voice_count]
        }
    }

    pub fn note_event(&mut self, engine: &mut Engine, note_event : NoteEvent) {

        let midi_num = note_event.note;
        let velocity = note_event.velocity;
        let on = note_event.down;
        let ts = note_event.timestamp;

        let targets = engine.get_control_map().note_receivers.clone();
        let mut vx = 0;
        let mut curr_voice : Option<usize> = None;
        let mut oldest_voice = 0;
        let mut oldest_ts = u64::max_value();

        for voice in self.voices.iter_mut () {
            println!("{},{}", voice.note.unwrap_or(0.0), voice.timestamp);
            if voice.timestamp < oldest_ts {
                oldest_ts = voice.timestamp;
                oldest_voice = vx;
            }
            if !on{
                if voice.note.unwrap_or(-1.0) == midi_num{
                    voice.note = None;
                    curr_voice = Some(vx);
                    break;
                }
            }else if voice.note == None {
                voice.note = Some(midi_num);
                voice.timestamp = ts;
                curr_voice = Some(vx);
                break;
            }
            vx += 1;
        }

        if curr_voice.is_none() {
            
            if !on {
                return;
            }
            let note = Note {
                ixs: targets[oldest_voice].clone().into_boxed_slice(),
                midi_num: self.voices[oldest_voice].note.unwrap(),
                velocity: velocity,
                on: false,
                timestamp: ts,
            };
            
            engine.send(Message::Note(note));
            self.voices[oldest_voice].note = Some(midi_num);
            curr_voice = Some(oldest_voice);
        }
        
        let note = Note {
            ixs: targets[curr_voice.unwrap_or(0)].clone().into_boxed_slice(),
            midi_num: midi_num,
            velocity: velocity,
            on: on,
            timestamp: ts,
        };
        
        engine.send(Message::Note(note));
    }



}


#[derive(Clone)]
pub struct NoteEvent {
    pub down: bool,
    pub note: f32,
    pub velocity: f32,
    pub timestamp : u64,
}

#[derive(Clone)]
struct Voice {
    pub note : Option<f32>,
    pub timestamp : u64,
}
