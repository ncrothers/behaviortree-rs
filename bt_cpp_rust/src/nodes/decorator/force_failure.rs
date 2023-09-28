use bt_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodePorts, NodeError, SyncNodeHalt, AsyncTick, AsyncNodeHalt},
};

/// The ForceFailureNode returns always Failure or Running
#[bt_node(DecoratorNode)]
pub struct ForceFailureNode {}

impl AsyncTick for ForceFailureNode {
    fn tick(&mut self) -> BoxFuture<Result<NodeStatus, NodeError>> {
        Box::pin(async move {
            self.set_status(NodeStatus::Running);
        
            let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick().await?;
        
            if child_status.is_completed() {
                self.reset_child().await;
        
                return Ok(NodeStatus::Failure);
            }
        
            Ok(child_status)
        })
    }
}

impl NodePorts for ForceFailureNode {}

impl AsyncNodeHalt for ForceFailureNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child();
        })
    }
}