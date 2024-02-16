use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{DecoratorNode, NodeError, NodeResult, TreeNodeDefaults},
};

/// The InverterNode returns Failure on Success, and Success on Failure
#[bt_node(
    node_type = DecoratorNode,
)]
pub struct InverterNode {}

#[bt_node(
    node_type = DecoratorNode,
    tick = tick,
    halt = halt,
)]
impl InverterNode {
    async fn tick(&mut self) -> NodeResult {
        node_.set_status(NodeStatus::Running);

        let child_status = node_.child.as_mut().unwrap().execute_tick().await?;

        match child_status {
            NodeStatus::Success => {
                node_.reset_child().await;
                Ok(NodeStatus::Failure)
            }
            NodeStatus::Failure => {
                node_.reset_child().await;
                Ok(NodeStatus::Success)
            }
            status @ (NodeStatus::Running | NodeStatus::Skipped) => Ok(status),
            NodeStatus::Idle => Err(NodeError::StatusError(
                "InverterNode".to_string(),
                "Idle".to_string(),
            )),
        }
    }

    async fn halt(&mut self) {
        node_.reset_child().await;
    }
}
