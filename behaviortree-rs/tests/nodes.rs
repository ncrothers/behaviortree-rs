use behaviortree_rs::{
    basic_types::{BTToString, NodeStatus, PortsList},
    macros::{define_ports, input_port},
    nodes::{
        action::{StatefulActionNode, SyncActionNode},
        NodeData, NodeResult,
    },
};
use behaviortree_rs_derive::{bt_node, BTToString, FromString};
use log::info;

#[derive(BTToString)]
struct Test {}

impl ToString for Test {
    fn to_string(&self) -> String {
        todo!()
    }
}

pub fn test_setup() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

#[derive(Debug)]
pub struct StatusNode;

impl SyncActionNode for StatusNode {
    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        let status: NodeStatus = ctx.config.get_input("status")?;

        info!("I am a node that returns {}!", status.bt_to_string());

        Ok(status)
    }

    fn ports(&self) -> PortsList {
        define_ports!(input_port!("status"))
    }
}

#[derive(Debug, Default)]
pub struct SuccessThenFailure {
    iter: usize,
}

impl SyncActionNode for SuccessThenFailure {
    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        let max_iters: usize = ctx.config.get_input("iters")?;

        info!("SuccessThenFailure!");

        if self.iter < max_iters {
            self.iter += 1;
            Ok(NodeStatus::Success)
        } else {
            Ok(NodeStatus::Failure)
        }
    }

    fn ports(&self) -> PortsList {
        define_ports!(input_port!("iters"))
    }
}

#[derive(Debug)]
pub struct EchoNode;

impl SyncActionNode for EchoNode {
    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        let msg: String = ctx.config.get_input("msg")?;

        info!("{msg}");

        Ok(NodeStatus::Success)
    }

    fn ports(&self) -> PortsList {
        define_ports!(input_port!("msg"))
    }
}

#[derive(Debug, Default)]
pub struct RunForNode {
    counter: usize,
}

impl StatefulActionNode for RunForNode {
    fn ports(&self) -> PortsList {
        define_ports!(
            input_port!("iters"),
            input_port!("status", NodeStatus::Success)
        )
    }

    fn on_start(&mut self, ctx: &mut NodeData) -> NodeResult {
        info!("on_start()");

        Ok(NodeStatus::Running)
    }

    fn on_running(&mut self, ctx: &mut NodeData) -> NodeResult {
        let limit: usize = ctx.config.get_input("iters")?;

        if self.counter < limit {
            info!("RunFor {}", self.counter);
            self.counter += 1;
            Ok(NodeStatus::Running)
        } else {
            Ok(ctx.config.get_input("status")?)
        }
    }
}

#[derive(Debug)]
pub struct DataNode {
    inner_name: String,
}

impl DataNode {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            inner_name: value.into(),
        }
    }
}

impl SyncActionNode for DataNode {
    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        Ok(NodeStatus::Success)
    }
}
