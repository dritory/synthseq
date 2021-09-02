// Copyright 2018 The Synthesizer IO Authors.
//
// MODIFIED by Endre Davoy
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

//! Interface for the audio engine.

use time;
use crate::config;

use synthesizer_io_core::graph::{IntoBoxedSlice, Message, Node, Note, SetParam};
use synthesizer_io_core::id_allocator::IdAllocator;
use synthesizer_io_core::module::Module;
use synthesizer_io_core::modules;
use synthesizer_io_core::queue::{Receiver, Sender};

/// The interface from the application to the audio engine.
///
/// It doesn't do the synthesis itself; the Worker (running in a real time
/// thread) handles that, but this module is responsible for driving
/// that process by sending messages.
pub struct Engine {
    core: Core,
    current_channel : usize,
    max_channels : usize,
    control_maps: [Option<ControlMap>; config::CHANNEL_COUNT],
}

/// Type used to identify nodes in the external interface (not to be confused
/// with nodes in the low-level graph).
pub type NodeId = usize;

/// The type of a module to be instantiated. It's not clear this should be
/// an enum, but it should do for now.
pub enum ModuleType {
    Sin,
    Saw,
}

/// The core owns the connection to the real-time worker.
struct Core {
    sample_rate: f32,
    rx: Receiver<Message>,
    tx: Sender<Message>,

    id_alloc: IdAllocator,

    monitor_queues: Option<MonitorQueues>,
}


#[derive(Clone)]
pub struct ControlMap {
    pub cutoff: usize,
    pub reso: usize,

    pub attack: usize,
    pub decay: usize,
    pub sustain: usize,
    pub release: usize,

    // node number of node that can be replaced to inject more audio
    pub ext: usize,

    pub note_receivers: [Vec<usize>; config::VOICE_COUNT],
}
const NONE_VEC_USIZE: Vec<usize> = vec![];
const NONE_CONTROL_MAP: Option<ControlMap> = None;

struct MonitorQueues {
    rx: Receiver<Vec<f32>>,
    tx: Sender<Vec<f32>>,
}

impl Engine {
    /// Create a new engine instance.
    ///
    /// This call takes ownership of channels to and from the worker.
    pub fn new(sample_rate: f32, rx: Receiver<Message>, tx: Sender<Message>) -> Engine {
        let core = Core::new(sample_rate, rx, tx);
        Engine {
            core: core,
            current_channel: 0,
            max_channels : 1,
            control_maps: [NONE_CONTROL_MAP; config::CHANNEL_COUNT],
        }
    }

    /// Initialize the engine with a simple mono synth.
    pub fn init_monosynth(&mut self) {
        self.max_channels = config::CHANNEL_COUNT;
        for c in 0..config::CHANNEL_COUNT{
            let mut control_map = self.core.init_controls();
            let (control_map, _) = self.core.init_monosynth(0, control_map);
            self.control_maps[c] = Some(control_map);
        }
    }
    /// Initialize the engine with a simple mono synth.
    pub fn init_polysynth(&mut self) {
        
        let mut ch_outputs: [usize;config::CHANNEL_COUNT] = [0;config::CHANNEL_COUNT];
        self.max_channels = config::CHANNEL_COUNT;
        for c in 0..config::CHANNEL_COUNT {
            let mut voice_outputs: [usize;config::VOICE_COUNT] = [0;config::VOICE_COUNT];
            let mut control_map = self.core.init_controls();

            for v in 0..config::VOICE_COUNT {
                let (c, o) = self.core.init_monosynth(v, control_map);
                control_map = c;
                voice_outputs[v] = o;
            }
            let id = self.core.id_alloc.alloc();
            self.core.update_sum_node(id, &voice_outputs);

            ch_outputs[c] = id;
            self.control_maps[c] = Some(control_map);
        }
        self.core.update_sum_node(0, &ch_outputs);
    }

    pub fn send(&self, msg: Message) {
        self.core.send(msg);
    }

    /// Poll the return queue. Right now this just returns the number of items
    /// retrieved.
    pub fn poll_rx(&mut self) -> usize {
        self.core.poll_rx()
    }

    /// Poll the monitor queue, retrieving audio data.
    pub fn poll_monitor(&mut self) -> Vec<f32> {
        self.core.poll_monitor()
    }

    /// Instantiate a module. Right now, the module has no inputs and the output
    /// is run directly to the output bus, but we'll soon add the ability to
    /// manipulate a wiring graph.
    ///
    /// Returns an id for the module's output. (TODO: will obviously need work for
    /// multi-output modules)
    pub fn instantiate_module(&mut self, node_id: NodeId, ty: ModuleType) -> usize {
        self.core.instantiate_module(node_id, ty)
    }

    /// Set the output bus.
    pub fn set_outputs(&mut self, outputs: &[usize]) {
        self.core.update_sum_node(0, outputs);
    }

    pub fn get_current_control_map(&self) -> ControlMap {
        let control_map = self.control_maps[self.current_channel].as_ref().unwrap().clone();
        control_map
    }

    pub fn get_control_map(&self, channel: usize) -> ControlMap {
        let control_map = self.control_maps[channel].as_ref().unwrap().clone();
        control_map
    }

    pub fn get_current_channel(&self) -> usize {
        self.current_channel
    }

    pub fn set_current_channel(&mut self, channel: usize){
        if channel < self.max_channels {
            self.current_channel = channel;
        }
    }
    pub fn set_ctrl_const(&mut self, value: f32, lo: f32, hi: f32, ix: usize,
        ts: u64)
    {
        let value = lo + value * (hi - lo);
        let param = SetParam {
            ix: ix,
            param_ix: 0,
            val: value,
            timestamp: ts,
        };
        self.send(Message::SetParam(param));
    }

}

impl Core {
    fn new(sample_rate: f32, rx: Receiver<Message>, tx: Sender<Message>) -> Core {
        let mut id_alloc = IdAllocator::new();
        id_alloc.reserve(0);
        let monitor_queues = None;
        Core {
            sample_rate,
            rx,
            tx,
            id_alloc,
            monitor_queues,
        }
    }

    pub fn create_node<
        B1: IntoBoxedSlice<(usize, usize)>,
        B2: IntoBoxedSlice<(usize, usize)>,
        M: Module + 'static,
    >(
        &mut self,
        module: M,
        in_buf_wiring: B1,
        in_ctrl_wiring: B2,
    ) -> usize {
        let id = self.id_alloc.alloc();
        self.send_node(Node::create(
            Box::new(module),
            id,
            in_buf_wiring,
            in_ctrl_wiring,
        ));
        id
    }
    fn init_controls(&mut self) -> ControlMap {
        let attack = self.create_node(modules::SmoothCtrl::new(5.0), [], []);
        let decay = self.create_node(modules::SmoothCtrl::new(5.0), [], []);
        let sustain = self.create_node(modules::SmoothCtrl::new(4.0), [], []);
        let release = self.create_node(modules::SmoothCtrl::new(5.0), [], []);
        let ext = self.create_node(modules::Sum::new(), [], []);
        let cutoff = self.create_node(modules::SmoothCtrl::new(880.0f32.log2()), [], []);
        let reso = self.create_node(modules::SmoothCtrl::new(0.5), [], []);
        ControlMap {
            cutoff,
            reso,
            attack,
            decay,
            sustain,
            release,
            ext,
            note_receivers: [NONE_VEC_USIZE; config::VOICE_COUNT],
        }
    }

    fn init_monosynth(
        &mut self,
        voice_number: usize,
        mut control_map: ControlMap,
    ) -> (ControlMap, usize) {
        let sample_rate = self.sample_rate;
        let note_pitch = self.create_node(modules::NotePitch::new(), [], []);
        let saw = self.create_node(modules::Saw::new(sample_rate), [], [(note_pitch, 0)]);

        let filter_out = self.create_node(
            modules::Biquad::new(sample_rate),
            [(saw, 0)],
            [(control_map.cutoff, 0), (control_map.reso, 0)],
        );
        let adsr = self.create_node(
            modules::Adsr::new(),
            [],
            vec![
                (control_map.attack, 0),
                (control_map.decay, 0),
                (control_map.sustain, 0),
                (control_map.release, 0),
            ],
        );

        let env_out = self.create_node(modules::Gain::new(), [(filter_out, 0)], [(adsr, 0)]);

        let ext_gain = self.create_node(modules::ConstCtrl::new(-2.0), [], []);
        let ext_atten = self.create_node(
            modules::Gain::new(),
            [(control_map.ext, 0)],
            [(ext_gain, 0)],
        );

        let monitor_in = self.create_node(modules::Sum::new(), [(env_out, 0), (ext_atten, 0)], []);

        let (monitor, tx, rx) = modules::Monitor::new();
        self.monitor_queues = Some(MonitorQueues { tx, rx });
        let monitor = self.create_node(monitor, [(monitor_in, 0)], []);

        control_map.note_receivers[voice_number].push(note_pitch);
        control_map.note_receivers[voice_number].push(adsr);

        (control_map, monitor)
    }

    fn send(&self, msg: Message) {
        self.tx.send(msg);
    }

    fn send_node(&mut self, node: Node) {
        self.send(Message::Node(node));
    }

    fn poll_rx(&mut self) -> usize {
        self.rx.recv().count()
    }

    fn poll_monitor(&self) -> Vec<f32> {
        let mut result = Vec::new();
        if let Some(ref qs) = self.monitor_queues {
            for mut item in qs.rx.recv_items() {
                result.extend_from_slice(&item);
                item.clear();
                qs.tx.send_item(item);
            }
        }
        result
    }

    fn update_sum_node(&mut self, sum_node: usize, outputs: &[usize]) {
        let module = Box::new(modules::Sum::new());
        let buf_wiring: Vec<_> = outputs.iter().map(|n| (*n, 0)).collect();
        self.send_node(Node::create(module, sum_node, buf_wiring, []));
    }

    fn instantiate_module(&mut self, _node_id: NodeId, ty: ModuleType) -> usize {
        let ll_id = match ty {
            ModuleType::Sin => {
                let pitch = self.create_node(modules::SmoothCtrl::new(440.0f32.log2()), [], []);
                let sample_rate = self.sample_rate;
                self.create_node(modules::Sin::new(sample_rate), [], [(pitch, 0)])
            }
            ModuleType::Saw => {
                let pitch = self.create_node(modules::SmoothCtrl::new(440.0f32.log2()), [], []);
                let sample_rate = self.sample_rate;
                self.create_node(modules::Saw::new(sample_rate), [], [(pitch, 0)])
            }
        };
        ll_id
    }
}
