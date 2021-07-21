

pub struct Synth_ctrl {

    pub synth : Synth<M, NFG,W, A, F, FW>,

};


impl Synth_ctrl {


fn init ()-> Self {
    synth = Synth::poly(());
    Self::new();
}

}