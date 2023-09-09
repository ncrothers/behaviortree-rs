use std::{cell::RefCell, rc::Rc};

use bt_cpp_rust::{nodes::{NodeConfig, NodeError, TreeNode, NodeHalt, StatefulActionNode}, basic_types::{NodeStatus, PortsList, BTToString}, macros::{define_ports, input_port, register_node}, tree::Factory, blackboard::Blackboard};
use bt_derive::{SyncActionNode, ActionNode, TreeNodeDefaults, StatefulActionNode};
use log::{info, error};


#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, SyncActionNode)]
pub struct StatusNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
}

impl StatusNode {
    pub fn new(name: &str, config: NodeConfig) -> StatusNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
        }
    }
}

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

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, SyncActionNode)]
pub struct SuccessThenFailure {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    iter: usize,
}

impl SuccessThenFailure {
    pub fn new(name: &str, config: NodeConfig) -> SuccessThenFailure {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
            iter: 0,
        }
    }
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

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, SyncActionNode)]
pub struct EchoNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
}

impl EchoNode {
    pub fn new(name: &str, config: NodeConfig) -> EchoNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
        }
    }
}

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

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, StatefulActionNode)]
pub struct RunForNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    counter: usize,
    halt_requested: RefCell<bool>,
}

impl RunForNode {
    pub fn new(name: &str, config: NodeConfig) -> RunForNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
            counter: 0,
            halt_requested: RefCell::new(false),
        }
    }
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