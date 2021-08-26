use crate::engine::{ControlMap, Engine};
use crate::config;
use synthesizer_io_core::graph::{Message, Note};

pub struct NoteModule {
    voices: [Voices; config::CHANNEL_COUNT],
}

impl NoteModule {
    pub fn new() -> NoteModule {
        NoteModule {
            voices: [NONE_VOICES; config::CHANNEL_COUNT]
        }
    }

    pub fn note_event(&mut self, engine: &mut Engine, note_event : NoteEvent, channel : usize) {

        let midi_num = note_event.note;
        let velocity = note_event.velocity;
        let on = note_event.down;
        let ts = note_event.timestamp;

        let targets = engine.get_control_map(channel).note_receivers.clone();
        let mut vx = 0;
        let mut curr_voice : Option<usize> = None;
        let mut oldest_voice = 0;
        let mut oldest_ts = u64::max_value();

        for voice in self.voices[channel].iter_mut () {
            //println!("{},{}", voice.note.unwrap_or(0.0), voice.timestamp);
            if voice.timestamp < oldest_ts {
                oldest_ts = voice.timestamp;
                oldest_voice = vx;
            }
            if !on{
                if voice.note.unwrap_or(-1.0) == midi_num{
                    voice.note = None;
                    voice.velocity = 0.0;
                    curr_voice = Some(vx);
                    break;
                }
            }else if voice.note == None {
                voice.note = Some(midi_num);
                voice.velocity = velocity;
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
                midi_num: self.voices[channel][oldest_voice].note.unwrap(),
                velocity: velocity,
                on: false,
                timestamp: ts,
            };
            
            engine.send(Message::Note(note));
            self.voices[channel][oldest_voice].note = Some(midi_num);
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


    pub fn get_voices(&self, channel : usize) -> &Voices{
        &self.voices[channel]
    }

}

#[derive(Clone)]
pub struct NoteEvent {
    pub down: bool,
    pub note: f32,
    pub velocity: f32,
    pub timestamp : u64,
}

pub const NONE_NOTE : NoteEvent = NoteEvent{down: false, note: 0.0, velocity: 0.0, timestamp: 0};

type Voices = [Voice; config::VOICE_COUNT];
const NONE_VOICE : Voice = Voice{note: None, velocity: 0.0, timestamp: 0};
const NONE_VOICES : [Voice; config::VOICE_COUNT] = [NONE_VOICE; config::VOICE_COUNT];

#[derive(Clone)]
pub struct Voice {
    pub note : Option<f32>,
    pub velocity: f32,
    pub timestamp : u64,
}
