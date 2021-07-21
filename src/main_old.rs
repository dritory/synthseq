//!
//!  test.rs.rs
//!
//!  Created by Mitchell Nordine at 05:57PM on December 19, 2014.
//!
//!  Always remember to run high performance Rust code with the --release flag. `Synth`
//!

extern crate cpal;
extern crate dasp;
extern crate pitch_calc as pitch;
extern crate synth;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midir::{Ignore, MidiInput};
use pitch::{Letter, LetterOctave, Step};
use std::error::Error;
use std::str;
use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};
use synth::{Envelope, Point};
use synth::dynamic::{oscillator, Mode, Oscillator};
// Currently supports i8, i32, f32.
pub type AudioSample = f32;
pub type Input = AudioSample;
pub type Output = AudioSample;

const CHANNELS: i32 = 2;
const SAMPLE_HZ: f64 = 44_100.0;

fn main() {
    run().expect("Error");
}

fn run() -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    let mut midi_in = MidiInput::new("midir reading input")?;
    midi_in.ignore(Ignore::None);

    let mut synth = {

        // The following envelopes should create a downward pitching sine wave that gradually quietens.
        // Try messing around with the points and adding some of your own!
        let amp_env = Envelope::from(vec!(
            //         Time ,  Amp ,  Curve
            Point::new(0.0  ,  1.0 ,  0.0),
            Point::new(1.0  ,  1.0 ,  0.0),
        ));
        let freq_env = Envelope::from(vec!(
            //         Time    , Freq   , Curve
            Point::new(0.0     , 0.0    , 0.0),
            Point::new(1.0     , 0.0    , 0.0),
        ));
        use synth::{Dynamic, oscillator::{amplitude, frequency, freq_warp}};
        // Now we can create our oscillator from our envelopes.
        // There are also Sine, Noise, NoiseWalk, SawExp and Square waveforms.
        let oscillator = Oscillator::new(oscillator::Waveform::Square, amplitude::Dynamic::Envelope(amp_env), frequency::Dynamic::Envelope(freq_env),freq_warp::Dynamic::None);

        // Here we construct our Synth from our oscillator.
        Dynamic::dynamic(Mode::poly())
            .oscillator(oscillator) // Add as many different oscillators as desired.
            .duration(6000.0) // Milliseconds.
            .base_pitch(LetterOctave(Letter::C, 1).hz()) // Hz.
            .loop_points(0.49, 0.51) // Loop start and end points.
            .fade(500.0, 500.0) // Attack and Release in milliseconds.
            .num_voices(16) // By default Synth is monophonic but this gives it `n` voice polyphony.
            .volume(0.2)
            .spread(0.0)

        // Other methods include:
            // .loop_start(0.0)
            // .loop_end(1.0)
            // .attack(ms)
            // .release(ms)
            // .note_freq_generator(nfg)
            // .oscillators([oscA, oscB, oscC])
            // .volume(1.0)
    };


    // let synth: Synth<
    //     synth::instrument::mode::Poly,
    //     (),
    //     synth::oscillator::waveform::Square,
    //     Envelope,
    //     Envelope,
    //     (),
    // > = Synth::poly(());
    let synth_arc = Arc::new(Mutex::new(synth));
    let synth_arc_1 = Arc::clone(&synth_arc);
    let synth_arc_2 = Arc::clone(&synth_arc);

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
        move |stamp, message, _| {
            println!("{}: {:?} (len = {})", stamp, message, message.len());
            let status = message[0];
            let data1 = message[1];
            let data2 = message[2];
            {
                let mut synth = synth_arc_1.lock().unwrap();
                match status {
                    144 => {
                        let note = Step(data1.into());
                        let freq = note.to_hz();
                        if data2 <= 0 {
                            synth.note_off(freq)
                        } else {
                            let vel = (data2 as f32) / 127.0;
                            synth.note_on(freq, vel);
                            print!("{}{}",freq.hz(), vel);
                        }
                    }
                    176 => {
                        if data1 == 7 {
                            let vel = (data2 as f32) / 127.0;
                            synth.set_volume(vel);
                        }
                    }
                    _ => (),
                }
            }
        },
        (),
    )?;
    println!("Connection open, reading input from '{}'", in_port_name);

    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config?!")
        .with_max_sample_rate();

    let config = supported_config.into();

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // react to stream events and read or write stream data here.
            let buffer: &mut [[f32; CHANNELS as usize]] =
                dasp::slice::to_frame_slice_mut(data).unwrap();

            dasp::slice::equilibrium(buffer);
            {
                let mut synth = synth_arc_2.lock().unwrap();
                synth.fill_slice(buffer, SAMPLE_HZ as f64);
            }
        },
        move |err| {
            // react to errors here.
        },
    )?;

    stream.play()?;
    while true{
        std::thread::sleep(std::time::Duration::from_millis(10000));
    }
    Ok(())
}

pub struct MidiMessage {
    pub freq: f32,
    pub vel: f32,
}
