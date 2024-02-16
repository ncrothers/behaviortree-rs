use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{DecoratorNode, NodeResult, TreeNodeDefaults},
};

/// The ForceSuccessNode returns always Success or Running
#[bt_node(
    node_type = DecoratorNode,
)]
pub struct ForceSuccessNode {}

#[bt_node(
    node_type = DecoratorNode,
    tick = tick,
    halt = halt,
)]
impl ForceSuccessNode {
    async fn tick(&mut self) -> NodeResult {
        node_.set_status(NodeStatus::Running);

        let child_status = node_.child.as_mut().unwrap().execute_tick().await?;

        if child_status.is_completed() {
            node_.reset_child().await;

            return Ok(NodeStatus::Success);
        }

        Ok(child_status)
    }

    async fn halt(&mut self) {
        node_.reset_child().await;
    }
}
