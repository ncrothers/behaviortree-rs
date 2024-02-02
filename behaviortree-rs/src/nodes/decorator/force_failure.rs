use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{AsyncHalt, AsyncTick, DecoratorNode, NodePorts, NodeResult, TreeNodeDefaults},
};

/// The ForceFailureNode returns always Failure or Running
#[bt_node(DecoratorNode)]
pub struct ForceFailureNode {}

impl AsyncTick for ForceFailureNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            self.set_status(NodeStatus::Running);

            let child_status = self
                .child
                .as_ref()
                .unwrap()
                .lock()
                .await
                .execute_tick()
                .await?;

            if child_status.is_completed() {
                self.reset_child().await;

                return Ok(NodeStatus::Failure);
            }

            Ok(child_status)
        })
    }
}

impl NodePorts for ForceFailureNode {}

impl AsyncHalt for ForceFailureNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child();
        })
    }
}
