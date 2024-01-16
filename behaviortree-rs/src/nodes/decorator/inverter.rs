use bt_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{
        AsyncHalt, AsyncTick, DecoratorNode, NodeError, NodePorts, NodeResult, TreeNodeDefaults,
    },
};

/// The InverterNode returns Failure on Success, and Success on Failure
#[bt_node(DecoratorNode)]
pub struct InverterNode {}

impl AsyncTick for InverterNode {
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

            match child_status {
                NodeStatus::Success => {
                    self.reset_child().await;
                    Ok(NodeStatus::Failure)
                }
                NodeStatus::Failure => {
                    self.reset_child().await;
                    Ok(NodeStatus::Success)
                }
                status @ (NodeStatus::Running | NodeStatus::Skipped) => Ok(status),
                NodeStatus::Idle => Err(NodeError::StatusError(
                    "InverterNode".to_string(),
                    "Idle".to_string(),
                )),
            }
        })
    }
}

impl NodePorts for InverterNode {}

impl AsyncHalt for InverterNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_child().await;
        })
    }
}
