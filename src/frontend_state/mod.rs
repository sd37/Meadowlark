use basedrop::{Shared, SharedCell};

pub mod nodes;
pub mod parameter;

pub use parameter::{
    coeff_to_db, db_to_coeff, Gradient, Param, ParamHandle, ParamType, Smooth, SmoothOutput,
    SmoothStatus, Unit,
};

use crate::graph_state::{GraphState, GraphStateManager, PortType};

pub struct FrontendState {
    graph_state: GraphStateManager,

    pub test_setup_sine_gen: Option<nodes::sine_gen::StereoSineGenNodeHandle>,
    pub test_setup_gain: Option<nodes::gain::GainNodeHandle>,
    pub test_setup_monitor: Option<nodes::monitor::StereoMonitorNodeHandle>,

    sample_rate: f32,
}

impl FrontendState {
    pub fn new(sample_rate: f32) -> (Self, Shared<SharedCell<GraphState>>) {
        let (graph_state, rt_graph_state) = GraphStateManager::new(sample_rate);

        let mut new_self = Self {
            graph_state,
            test_setup_sine_gen: None,
            test_setup_gain: None,
            test_setup_monitor: None,
            sample_rate,
        };

        new_self.test_setup();

        (new_self, rt_graph_state)
    }

    /// A temporary test setup: "sine wave generator" -> "gain knob" -> "db meter".
    pub fn test_setup(&mut self) {
        let sine_gen_id = String::from("sine_gen");
        let (sine_gen_node, sine_gen_node_handle) = nodes::sine_gen::StereoSineGenNode::new(
            440.0,
            0.0,
            -90.0,
            0.0,
            self.sample_rate,
            self.graph_state.coll_handle(),
        );

        let gain_id = String::from("gain");
        let (gain_node, gain_node_handle) = nodes::gain::StereoGainNode::new(
            0.0,
            -90.0,
            3.0,
            self.sample_rate,
            self.graph_state.coll_handle(),
        );

        let monitor_id = String::from("monitor");
        let (monitor_node, monitor_node_handle) =
            nodes::monitor::StereoMonitorNode::new(2048, true, &self.graph_state.coll_handle());

        self.graph_state.modify_graph(|mut graph| {
            graph
                .add_new_node(&sine_gen_id, Box::new(sine_gen_node))
                .unwrap();
            graph.add_new_node(&gain_id, Box::new(gain_node)).unwrap();
            graph
                .add_new_node(&monitor_id, Box::new(monitor_node))
                .unwrap();

            graph
                .add_port_connection(PortType::StereoAudio, &sine_gen_id, 0, &gain_id, 0)
                .unwrap();

            graph
                .add_port_connection(PortType::StereoAudio, &gain_id, 0, &monitor_id, 0)
                .unwrap();
        });

        self.test_setup_sine_gen = Some(sine_gen_node_handle);
        self.test_setup_gain = Some(gain_node_handle);
        self.test_setup_monitor = Some(monitor_node_handle);
    }

    /// Call periodically to collect garbage in the rt thread.
    ///
    /// TODO: Actually do this somewhere!
    pub fn collect(&mut self) {
        self.graph_state.collect();
    }
}
