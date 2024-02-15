use behaviortree_rs::{
    basic_types::{BTToString, NodeStatus, PortsList},
    macros::{define_ports, input_port},
    nodes::{AsyncStatefulActionNode, NodeResult},
};
use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;
use log::info;

pub fn test_setup() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

#[bt_node(
    node_type = SyncActionNode,
    ports = provided_ports,
    tick = tick,
)]
pub struct StatusNode {}

impl StatusNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let status: NodeStatus = self.config.get_input("status")?;

            info!("I am a node that returns {}!", status.bt_to_string());

            Ok(status)
        })
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("status"))
    }
}

#[bt_node(
    node_type = SyncActionNode,
    ports = provided_ports,
    tick = tick,
)]
pub struct SuccessThenFailure {
    #[bt(default)]
    iter: usize,
}

impl SuccessThenFailure {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let max_iters: usize = self.config.get_input("iters")?;

            info!("SuccessThenFailure!");

            if self.iter < max_iters {
                self.iter += 1;
                Ok(NodeStatus::Success)
            } else {
                Ok(NodeStatus::Failure)
            }
        })
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("iters"))
    }
}

#[bt_node(
    node_type = SyncActionNode,
    runtime = Async,
    ports = provided_ports,
    tick = tick,
)]
pub struct EchoNode {}

impl EchoNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let msg: String = self.config.get_input("msg")?;

            info!("{msg}");

            Ok(NodeStatus::Success)
        })
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("msg"))
    }
}

#[bt_node(
    node_type = StatefulActionNode,
    on_start = on_start,
    on_running = on_running,
    ports = provided_ports,
)]
pub struct RunForNode {
    #[bt(default)]
    counter: usize,
}

impl RunForNode {
    fn provided_ports(&self) -> PortsList {
        define_ports!(
            input_port!("iters"),
            input_port!("status", NodeStatus::Success)
        )
    }

    fn on_start(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            info!("on_start()");

            Ok(NodeStatus::Running)
        })
    }

    fn on_running(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let limit: usize = self.config.get_input("iters")?;

            if self.counter < limit {
                info!("RunFor {}", self.counter);
                self.counter += 1;
                Ok(NodeStatus::Running)
            } else {
                Ok(self.config.get_input("status")?)
            }
        })
    }
}

#[bt_node(
    node_type = SyncActionNode,
    tick = tick,
)]
pub struct DataNode {
    inner_name: String,
}

impl DataNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move { Ok(NodeStatus::Success) })
    }
}
