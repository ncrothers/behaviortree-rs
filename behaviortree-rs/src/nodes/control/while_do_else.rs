use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{AsyncHalt, AsyncTick, ControlNode, NodeError, NodePorts, NodeResult},
};

/// WhileDoElse must have exactly 2 or 3 children.
/// It is a REACTIVE node of IfThenElseNode.
///
/// The first child is the "statement" that is executed at each tick
///
/// If result is SUCCESS, the second child is executed.
///
/// If result is FAILURE, the third child is executed.
///
/// If the 2nd or 3d child is RUNNING and the statement changes,
/// the RUNNING child will be stopped before starting the sibling.
#[bt_node(ControlNode)]
pub struct WhileDoElseNode {}

impl AsyncTick for WhileDoElseNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            let children_count = self.children.len();
            // Node should only have 2 or 3 children
            if !(2..=3).contains(&children_count) {
                return Err(NodeError::NodeStructureError(
                    "IfThenElseNode must have either 2 or 3 children.".to_string(),
                ));
            }

            self.status = NodeStatus::Running;

            let condition_status = self.children[0].execute_tick().await?;

            if matches!(condition_status, NodeStatus::Running) {
                return Ok(NodeStatus::Running);
            }

            let mut status = NodeStatus::Idle;

            match condition_status {
                NodeStatus::Success => {
                    if children_count == 3 {
                        self.halt_child(2).await?;
                    }

                    status = self.children[1].execute_tick().await?;
                }
                NodeStatus::Failure => match children_count {
                    3 => {
                        self.halt_child(1).await?;
                        status = self.children[2].execute_tick().await?;
                    }
                    2 => {
                        status = NodeStatus::Failure;
                    }
                    _ => {}
                },
                _ => {}
            }

            match status {
                NodeStatus::Running => Ok(NodeStatus::Running),
                status => {
                    self.reset_children().await;
                    Ok(status)
                }
            }
        })
    }
}

impl NodePorts for WhileDoElseNode {}

impl AsyncHalt for WhileDoElseNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.reset_children().await;
        })
    }
}
