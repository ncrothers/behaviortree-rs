use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{DecoratorNode, NodeResult, TreeNodeDefaults},
};

/// The ForceFailureNode returns always Failure or Running
#[bt_node(
    node_type = DecoratorNode,
)]
pub struct ForceFailureNode {}

#[bt_node(
    node_type = DecoratorNode,
    tick = tick,
    halt = halt,
)]
impl ForceFailureNode {
    async fn tick(&mut self) -> NodeResult {
        node_.set_status(NodeStatus::Running);

        let child_status = node_.child.as_mut().unwrap().execute_tick().await?;

        if child_status.is_completed() {
            node_.reset_child().await;

            return Ok(NodeStatus::Failure);
        }

        Ok(child_status)
    }

    async fn halt(&mut self) {
        node_.reset_child().await;
    }
}
