
use module::{Module, Buffer};

pub struct PolyNote {
    tones: &[f32],
}

impl PolyNote {
    pub fn new(voice_count : usize) -> PolyNote {
        let tones : [f32; voice_count] = [0.0; voice_count];
        PolyNote { value: &[tones]) }
    }
}

impl Module for PolyNote {
    fn n_ctrl_out(&self) -> usize { 1 }

    fn handle_note(&mut self, midi_num: f32, _velocity: f32, on: bool) {
        if on {
            self.value = midi_num * (1.0 / 12.0) + (440f32.log2() - 69.0 / 12.0);
        }
    }

    fn process(&mut self, _control_in: &[f32], control_out: &mut [f32],
        _buf_in: &[&Buffer], _buf_out: &mut [Buffer])
    {
        control_out[0] = self.value;
    }
}
