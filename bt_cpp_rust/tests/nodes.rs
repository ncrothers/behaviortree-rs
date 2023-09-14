use std::cell::RefCell;

use bt_cpp_rust::{nodes::{NodeError, TreeNode, NodeHalt, StatefulActionNode}, basic_types::{NodeStatus, PortsList, BTToString}, macros::{define_ports, input_port}};
use bt_derive::bt_node;
use log::info;

pub fn test_setup() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();
}

#[bt_node(SyncActionNode)]
pub struct StatusNode {}

impl TreeNode for StatusNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let status: NodeStatus = self.config.get_input("status")?;

        info!("I am a node that returns {}!", status.bt_to_string());

        Ok(status)
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("status"))
    }
}

impl NodeHalt for StatusNode {}

#[bt_node(SyncActionNode)]
pub struct SuccessThenFailure {
    #[bt(default)]
    iter: usize,
}

impl TreeNode for SuccessThenFailure {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let max_iters: usize = self.config.get_input("iters")?;

        info!("SuccessThenFailure!");

        if self.iter < max_iters {
            self.iter += 1;
            Ok(NodeStatus::Success)
        }
        else {
            Ok(NodeStatus::Failure)
        }
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("iters"))
    }
}

impl NodeHalt for SuccessThenFailure {}

#[bt_node(SyncActionNode)]
pub struct EchoNode {}

impl TreeNode for EchoNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let msg: String = self.config.get_input("msg")?;

        info!("{msg}");

        Ok(NodeStatus::Success)
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("msg"))
    }
}

impl NodeHalt for EchoNode {}

#[bt_node(StatefulActionNode)]
pub struct RunForNode {
    #[bt(default)]
    counter: usize,
    #[bt(default = "RefCell::new(false)")]
    halt_requested: RefCell<bool>,
}

impl TreeNode for RunForNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Idle)
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("iters"), input_port!("status", NodeStatus::Success))
    }
}

impl StatefulActionNode for RunForNode {
    fn on_start(&mut self) -> Result<NodeStatus, NodeError> {
        info!("on_start()");

        Ok(NodeStatus::Running)
    }

    fn on_running(&mut self) -> Result<NodeStatus, NodeError> {
        let limit: usize = self.config.get_input("iters")?;

        if self.counter < limit {
            info!("RunFor {}", self.counter);
            self.counter += 1;
            Ok(NodeStatus::Running)
        }
        else {
            Ok(self.config.get_input("status")?)
        }
    }
}