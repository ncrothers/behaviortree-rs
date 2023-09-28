use bt_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodePorts, NodeError, SyncNodeHalt, AsyncNodeHalt, AsyncTick},
};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[bt_node(DecoratorNode)]
pub struct KeepRunningUntilFailureNode {}

impl AsyncTick for KeepRunningUntilFailureNode {
    fn tick(&mut self) -> BoxFuture<Result<NodeStatus, NodeError>> {
        Box::pin(async move {
            self.set_status(NodeStatus::Running);
        
            let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick().await?;
        
            match child_status {
                NodeStatus::Success => {
                    self.reset_child();
                    Ok(NodeStatus::Running)
                }
                NodeStatus::Failure => {
                    self.reset_child();
                    Ok(NodeStatus::Failure)
                }
                _ => Ok(NodeStatus::Running)
            }
        })
    }
}

impl NodePorts for KeepRunningUntilFailureNode {}

impl AsyncNodeHalt for KeepRunningUntilFailureNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child();
        })
    }
}