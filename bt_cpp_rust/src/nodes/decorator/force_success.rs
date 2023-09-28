use bt_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodePorts, NodeError, SyncNodeHalt, AsyncTick, AsyncNodeHalt},
};

/// The ForceSuccessNode returns always Success or Running
#[bt_node(DecoratorNode)]
pub struct ForceSuccessNode {}

impl AsyncTick for ForceSuccessNode {
    fn tick(&mut self) -> BoxFuture<Result<NodeStatus, NodeError>> {
        Box::pin(async move {
            self.set_status(NodeStatus::Running);
        
            let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick().await?;
        
            if child_status.is_completed() {
                self.reset_child();
        
                return Ok(NodeStatus::Success);
            }
        
            Ok(child_status)
        })
    }
}

impl NodePorts for ForceSuccessNode {}

impl AsyncNodeHalt for ForceSuccessNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child();
        })
    }
}