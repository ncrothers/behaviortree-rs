use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{AsyncHalt, AsyncTick, DecoratorNode, NodePorts, NodeResult, TreeNodeDefaults},
};

/// The ForceFailureNode returns always Failure or Running
#[bt_node(
    node_type = DecoratorNode,
    tick = tick,
    halt = halt,
)]
pub struct ForceFailureNode {}

impl ForceFailureNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            self.set_status(NodeStatus::Running);

            let child_status = self.child.as_mut().unwrap().execute_tick().await?;

            if child_status.is_completed() {
                self.reset_child().await;

                return Ok(NodeStatus::Failure);
            }

            Ok(child_status)
        })
    }
    
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child().await;
        })
    }
}
