use behaviortree_rs::{
    basic_types::{BTToString, NodeStatus, PortsList},
    macros::{define_ports, input_port},
    nodes::NodeResult,
};
use behaviortree_rs_derive::bt_node;
use log::info;

pub fn test_setup() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

#[bt_node(SyncActionNode)]
pub struct StatusNode {}

#[bt_node(
    node_type = SyncActionNode,
    ports = provided_ports,
    tick = tick,
)]
impl StatusNode {
    async fn tick(&mut self) -> NodeResult {
        let status: NodeStatus = node_.config.get_input("status")?;

        info!("I am a node that returns {}!", status.bt_to_string());

        Ok(status)
    }

    fn provided_ports() -> PortsList {
        define_ports!(input_port!("status"))
    }
}

#[bt_node(SyncActionNode)]
pub struct SuccessThenFailure {
    #[bt(default)]
    iter: usize,
}

#[bt_node(
    node_type = SyncActionNode,
    ports = provided_ports,
    tick = tick,
)]
impl SuccessThenFailure {
    async fn tick(&mut self) -> NodeResult {
        let max_iters: usize = node_.config.get_input("iters")?;

        info!("SuccessThenFailure!");

        if self.iter < max_iters {
            self.iter += 1;
            Ok(NodeStatus::Success)
        } else {
            Ok(NodeStatus::Failure)
        }
    }

    fn provided_ports() -> PortsList {
        define_ports!(input_port!("iters"))
    }
}

#[bt_node(SyncActionNode)]
pub struct EchoNode {}

#[bt_node(
    node_type = SyncActionNode,
    ports = provided_ports,
    tick = tick,
)]
impl EchoNode {
    async fn tick(&mut self) -> NodeResult {
        let msg: String = node_.config.get_input("msg")?;

        info!("{msg}");

        Ok(NodeStatus::Success)
    }

    fn provided_ports() -> PortsList {
        define_ports!(input_port!("msg"))
    }
}

#[bt_node(StatefulActionNode)]
pub struct RunForNode {
    #[bt(default)]
    counter: usize,
}

#[bt_node(
    node_type = StatefulActionNode,
    on_start = on_start,
    on_running = on_running,
    ports = provided_ports,
)]
impl RunForNode {
    fn provided_ports() -> PortsList {
        define_ports!(
            input_port!("iters"),
            input_port!("status", NodeStatus::Success)
        )
    }

    async fn on_start(&mut self) -> NodeResult {
        info!("on_start()");

        Ok(NodeStatus::Running)
    }

    async fn on_running(&mut self) -> NodeResult {
        let limit: usize = node_.config.get_input("iters")?;

        if self.counter < limit {
            info!("RunFor {}", self_.counter);
            self.counter += 1;
            Ok(NodeStatus::Running)
        } else {
            Ok(node_.config.get_input("status")?)
        }
    }
}

#[bt_node(SyncActionNode)]
pub struct DataNode {
    inner_name: String,
}

#[bt_node(
    node_type = SyncActionNode,
    tick = tick,
)]
impl DataNode {
    async fn tick(&mut self) -> NodeResult {
        Ok(NodeStatus::Success)
    }
}
