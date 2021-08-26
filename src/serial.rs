use serialport;
use std::time::Duration;
use crate::engine::{Engine, ControlMap};
use crate::note::NoteModule;
use std::io::{self};

pub struct Serial {

}

impl Serial {
    pub fn new() -> Serial {
        Serial {}
    }
    pub fn dispatch_serial(&mut self, note_module : &mut NoteModule, engine: &mut Engine, serial_buf: &[u8], ts: u64) {
        let mut i = 0;
        let current_channel = engine.get_current_channel();
        let control_map : ControlMap = engine.get_current_control_map();
        
        println!("{:?}",serial_buf);
    }
}

fn main() {
    let ports = serialport::available_ports().expect("No ports found!");
    for p in ports {
        println!("{}", p.port_name);
    }
    let mut port = serialport::new("COM7", 115200)
    .timeout(Duration::from_millis(10))
    .open().expect("Failed to open port");


    loop {
        let mut serial_buf: Vec<u8> = vec![0; 5];
        let result = port.read(serial_buf.as_mut_slice());

        let result = match result{
            Ok(file) => file,
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => 0,
            Err(error) => {println!("{}", error); 0},
        };
        if result > 0 {
            println!("{:?}",serial_buf);
        }
        std::thread::sleep(Duration::from_millis(50));
    }

}
