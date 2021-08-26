
use crate::engine::{ControlMap, Engine};
use crate::note::{NoteModule, NoteEvent};

use synthesizer_io_core::graph::{Message, SetParam};

pub struct Midi {

}


impl Midi {
    pub fn new() -> Midi {
        Midi {}
    }

    fn set_ctrl_const(&mut self, engine: &mut Engine, value: u8, lo: f32, hi: f32, ix: usize,
        ts: u64)
    {
        let value = lo + value as f32 * (1.0/127.0) * (hi - lo);
        let param = SetParam {
            ix: ix,
            param_ix: 0,
            val: value,
            timestamp: ts,
        };
        engine.send(Message::SetParam(param));
    }

    pub fn dispatch_midi(&mut self, note_module : &mut NoteModule, engine: &mut Engine, data: &[u8], ts: u64) {
        let mut i = 0;
        let channel = engine.get_current_channel();
        let control_map : ControlMap = engine.get_current_control_map();


        while i < data.len() {
            println!("{},{},{}", data[i], data[i + 1], data[i + 2]);
            if data[i] == 0xb0 {
                let controller = data[i + 1];
                let value = data[i + 2];
                
                match controller {
                    1 => {
                        let cutoff = control_map.cutoff;
                        self.set_ctrl_const(engine, value, 0.0, 22_000f32.log2(), cutoff, ts);
                    }
                    2 => {
                        let reso = control_map.reso;
                        self.set_ctrl_const(engine, value, 0.0, 0.995, reso, ts);
                    }

                    5 => {
                        let attack = control_map.attack;
                        self.set_ctrl_const(engine, value, 0.0, 10.0, attack, ts);
                    }
                    6 => {
                        let decay = control_map.decay;
                        self.set_ctrl_const(engine, value, 0.0, 10.0, decay, ts);
                    }
                    7 => {
                        let sustain = control_map.sustain;
                        self.set_ctrl_const(engine, value, 0.0, 6.0, sustain, ts);
                    }
                    8 => {
                        let release = control_map.release;
                        self.set_ctrl_const(engine, value, 0.0, 10.0, release, ts);
                    }
                    _ => println!("don't have handler for controller {}", controller),
                }
                i += 3;
            } else if data[i] == 0x90 || data[i] == 0x80 {
                let midi_num = data[i + 1] as f32;
                let velocity = data[i + 2] as f32;
                let on = data[i] == 0x90 && velocity > 0.0;
                let note_event = NoteEvent{down: on, note : midi_num, velocity : velocity, timestamp: ts};

                note_module.note_event(engine, note_event, channel);
                i += 3;
            } else {
                println!("don't have handler for midi code {}", data[i]);
                break;
            }
        }
    }

}
