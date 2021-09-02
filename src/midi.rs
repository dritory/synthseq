
use crate::engine::{ControlMap, Engine};
use crate::note::{NoteModule, NoteEvent};


use midir::{MidiInput, MidiInputPort, MidiInputConnection, ConnectError};
use synthesizer_io_core::graph::{Message, SetParam};

use std::io::{stdin, stdout, Write};


use std::sync::{Arc, Mutex, mpsc};

pub struct Midi {
}


impl Midi {
    pub fn new() -> Midi {
        Midi { }
    }

    pub fn dispatch_midi(note_module : &mut NoteModule, engine: &mut Engine, data: &[u8], ts: u64) {
        let mut i = 0;
        let channel = engine.get_current_channel();
        let control_map : ControlMap = engine.get_current_control_map();
        
        while i < data.len() {
            println!("{},{},{}", data[i], data[i + 1], data[i + 2]);
            if data[i] == 0xb0 {
                let controller = data[i + 1];
                let value = Midi::midi_value_to_float(data[i + 2]);
                
                match controller {
                    1 => {
                        let cutoff = control_map.cutoff;
                        engine.set_ctrl_const(value, 0.0, 22_000f32.log2(), cutoff, ts);
                    }
                    2 => {
                        let reso = control_map.reso;
                        engine.set_ctrl_const(value, 0.0, 0.995, reso, ts);
                    }

                    5 => {
                        let attack = control_map.attack;
                        engine.set_ctrl_const(value, 0.0, 10.0, attack, ts);
                    }
                    6 => {
                        let decay = control_map.decay;
                        engine.set_ctrl_const(value, 0.0, 10.0, decay, ts);
                    }
                    7 => {
                        let sustain = control_map.sustain;
                        engine.set_ctrl_const(value, 0.0, 6.0, sustain, ts);
                    }
                    8 => {
                        let release = control_map.release;
                        engine.set_ctrl_const(value, 0.0, 10.0, release, ts);
                    }
                    _ => println!("don't have handler for controller {}", controller),
                }
                i += 3;
            } else if data[i] == 0x90 || data[i] == 0x80 {
                let midi_num = data[i + 1] as f32;
                let velocity = data[i + 2] as f32;
                let on = data[i] == 0x90 && velocity > 0.0;
                let note_event = NoteEvent{down: on, note : midi_num, velocity : velocity, timestamp: ts};

                note_module.note_event(engine, note_event, 0);
                i += 3;
            } else {
                println!("don't have handler for midi code {}", data[i]);
                break;
            }
        }
    }

    pub fn find_midi_port(midi_in : &MidiInput) -> Result<MidiInputPort, &str>{
        let in_ports = midi_in.ports();
        match in_ports.len() {
            0 => return Err("no input port found"),
            1 => {
                println!(
                    "Choosing the only available input port: {}",
                    midi_in.port_name(&in_ports[0]).unwrap()
                );
                Ok(in_ports[0].clone())
            }
            _ => {
                println!("\nAvailable input ports:");
                for (i, p) in in_ports.iter().enumerate() {
                    println!("{}: {}", i, midi_in.port_name(p).unwrap());
                }
                print!("Please select input port: ");
                stdout().flush().expect("Failed to flush stdout");
                let mut input = String::new();
                stdin().read_line(&mut input).expect("Failed to read line");
                Ok(in_ports
                    .get(input.trim().parse::<usize>().expect("Failed to parse input"))
                    .expect("invalid input port selected").clone())
            }
        }
    }
    pub fn setup_midi_connection(){
        
    }

    fn midi_value_to_float(value : u8) -> f32 {
        value as f32 * (1.0/127.0) 
    }
}
