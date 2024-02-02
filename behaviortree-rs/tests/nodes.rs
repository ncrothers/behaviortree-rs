use behaviortree_rs::{
    basic_types::{BTToString, NodeStatus, PortsList},
    macros::{define_ports, input_port},
    nodes::{AsyncHalt, AsyncStatefulActionNode, AsyncTick, NodePorts, NodeResult},
};
use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;
use log::info;

pub fn test_setup() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();
}

#[bt_node(SyncActionNode)]
pub struct StatusNode {}

impl AsyncTick for StatusNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let status: NodeStatus = self.config.get_input("status").await?;

            info!("I am a node that returns {}!", status.bt_to_string());

            Ok(status)
        })
    }
}

impl NodePorts for StatusNode {
    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("status"))
    }
}

impl AsyncHalt for StatusNode {}

#[bt_node(SyncActionNode)]
pub struct SuccessThenFailure {
    #[bt(default)]
    iter: usize,
}

impl AsyncTick for SuccessThenFailure {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let max_iters: usize = self.config.get_input("iters").await?;

            info!("SuccessThenFailure!");

            if self.iter < max_iters {
                self.iter += 1;
                Ok(NodeStatus::Success)
            } else {
                Ok(NodeStatus::Failure)
            }
        })
    }
}

impl NodePorts for SuccessThenFailure {
    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("iters"))
    }
}

impl AsyncHalt for SuccessThenFailure {}

#[bt_node(SyncActionNode, Async)]
pub struct EchoNode {}

impl AsyncTick for EchoNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let msg: String = self.config.get_input("msg").await?;

            info!("{msg}");

            Ok(NodeStatus::Success)
        })
    }
}

impl NodePorts for EchoNode {
    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("msg"))
    }
}

impl AsyncHalt for EchoNode {}

#[bt_node(StatefulActionNode)]
pub struct RunForNode {
    #[bt(default)]
    counter: usize,
}

impl NodePorts for RunForNode {
    fn provided_ports(&self) -> PortsList {
        define_ports!(
            input_port!("iters"),
            input_port!("status", NodeStatus::Success)
        )
    }
}

impl AsyncStatefulActionNode for RunForNode {
    fn on_start(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            info!("on_start()");

            Ok(NodeStatus::Running)
        })
    }

    fn on_running(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let limit: usize = self.config.get_input("iters").await?;

            if self.counter < limit {
                info!("RunFor {}", self.counter);
                self.counter += 1;
                Ok(NodeStatus::Running)
            } else {
                Ok(self.config.get_input("status").await?)
            }
        })
    }
}

#[bt_node(SyncActionNode)]
pub struct DataNode {
    inner_name: String,
}

impl NodePorts for DataNode {}

impl AsyncTick for DataNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move { Ok(NodeStatus::Success) })
    }
}

impl AsyncHalt for DataNode {}
