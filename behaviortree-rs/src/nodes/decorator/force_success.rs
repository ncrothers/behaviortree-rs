use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{AsyncHalt, AsyncTick, DecoratorNode, NodePorts, NodeResult, TreeNodeDefaults},
};

/// The ForceSuccessNode returns always Success or Running
#[bt_node(DecoratorNode)]
pub struct ForceSuccessNode {}

impl AsyncTick for ForceSuccessNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            self.set_status(NodeStatus::Running);

            let child_status = self.child.as_mut().unwrap().execute_tick().await?;

            if child_status.is_completed() {
                self.reset_child().await;

                return Ok(NodeStatus::Success);
            }

            Ok(child_status)
        })
    }
}

impl NodePorts for ForceSuccessNode {}

impl AsyncHalt for ForceSuccessNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child();
        })
    }
}
