use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{AsyncHalt, AsyncTick, DecoratorNode, NodePorts, NodeResult, TreeNodeDefaults},
};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[bt_node(DecoratorNode)]
pub struct KeepRunningUntilFailureNode {}

impl AsyncTick for KeepRunningUntilFailureNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            self.set_status(NodeStatus::Running);

            let child_status = self.child.as_mut().unwrap().execute_tick().await?;

            match child_status {
                NodeStatus::Success => {
                    self.reset_child().await;
                    Ok(NodeStatus::Running)
                }
                NodeStatus::Failure => {
                    self.reset_child().await;
                    Ok(NodeStatus::Failure)
                }
                _ => Ok(NodeStatus::Running),
            }
        })
    }
}

impl NodePorts for KeepRunningUntilFailureNode {}

impl AsyncHalt for KeepRunningUntilFailureNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child().await;
        })
    }
}
