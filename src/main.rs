// Copyright 2017 The Synthesizer IO Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(target_os = "macos")]
extern crate coreaudio;
#[cfg(target_os = "macos")]
extern crate coremidi;

#[cfg(not(target_os = "macos"))]
extern crate cpal;
#[cfg(not(target_os = "macos"))]
extern crate midir;

extern crate time;

extern crate synthesizer_io_core;

mod engine;
mod midi;
mod note;


#[cfg(not(target_os = "macos"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleRate;

#[cfg(not(target_os = "macos"))]
use midir::MidiInput;
#[cfg(not(target_os = "macos"))]

use synthesizer_io_core::modules;

use synthesizer_io_core::graph::{IntoBoxedSlice, Message, Node, Note, SetParam};
use synthesizer_io_core::module::{Module, N_SAMPLES_PER_CHUNK};
use synthesizer_io_core::queue::Sender;
use synthesizer_io_core::worker::Worker;
use std::error::Error;

use std::sync::{Arc, Mutex};
use std::io::{stdin, stdout, Write};


use engine::Engine;
use midi::Midi;
use note::NoteModule;

const MAX_NODES : usize = 255;

const VOICE_COUNT : usize = 16;
const CHANNEL_COUNT : usize = 8;
const CHANNELS: i32 = 2;
const SAMPLE_HZ: f32 = 48000.0;

fn main() {
    let (worker, tx, rx) = Worker::create(4096);

    let mut engine = Engine::new(SAMPLE_HZ, rx, tx);
    engine.init_polysynth(CHANNEL_COUNT, VOICE_COUNT);
    let engine = Arc::new(Mutex::new(engine));

    let mut note_module = NoteModule::new(VOICE_COUNT);
    let note_module = Arc::new(Mutex::new(note_module));
    
    std::thread::spawn(move || {
        run_midi(note_module, engine);
    }); 

    run_cpal(worker);
}


fn run_midi( note_module : Arc<Mutex<NoteModule>>, engine : Arc<Mutex<Engine>>){
    // midi setup
    let mut midi = Midi::new();

    let mut midi_in = MidiInput::new("midir input").expect("can't create midi input");
    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => return Err("no input port found").unwrap(),
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
            stdout().flush().expect("Failed to flush stdout");
            let mut input = String::new();
            stdin().read_line(&mut input).expect("Failed to read line");
            in_ports
                .get(input.trim().parse::<usize>().expect("Failed to parse input"))
                .expect("invalid input port selected")
        }
    };
    midi_in.ignore(::midir::Ignore::None);
    let result = midi_in.connect(
        in_port,
        "midir-read-input",
        move |ts, data, _| {
            let mut engine = engine.lock().unwrap();
            let mut note_module = note_module.lock().unwrap();
            midi.dispatch_midi(&mut *note_module, &mut *engine, data, ts);
        }, 
        (),
    );
    if let Err(e) = result {
        println!("error connecting to midi: {:?}", e);
    }
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10000));
    }
}

#[cfg(not(target_os = "macos"))]
fn run_cpal(mut worker: Worker) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config?!")
        .with_sample_rate(SampleRate(SAMPLE_HZ as u32));

    let config = supported_config.into();
    println!("Format: {:?}",config);
    
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
            //info.timestamp().callback().unwrap()
            let mut buf_slice = data;
            let mut i = 0;
            let mut timestamp = time::precise_time_ns();
            while i < buf_slice.len() {
                // should let the graph generate stereo
                let buf = worker.work(timestamp)[0].get();
                for j in 0..N_SAMPLES_PER_CHUNK {
                    buf_slice[i + j * 2] = buf[j];
                    buf_slice[i + j * 2 + 1] = buf[j];
                }

                // TODO: calculate properly, magic value is 64 * 1e9 / 44_100
                timestamp += 1451247 * (N_SAMPLES_PER_CHUNK as u64) / 64;
                i += N_SAMPLES_PER_CHUNK * 2;
            }
                
               
            
        },
        move |err| {
            // react to errors here.
        },
    ).expect("Failed to build audio stream");

    stream.play().expect("Failed to play stream");
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10000));
    }
}