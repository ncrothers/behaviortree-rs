use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;
use log::warn;

use crate::{
    basic_types::NodeStatus,
    nodes::{AsyncHalt, AsyncTick, ControlNode, NodeError, NodePorts, NodeResult},
};

/// IfThenElseNode must have exactly 2 or 3 children. This node is NOT reactive.
///
/// The first child is the "statement" of the if.
///
/// If that return SUCCESS, then the second child is executed.
///
/// Instead, if it returned FAILURE, the third child is executed.
///
/// If you have only 2 children, this node will return FAILURE whenever the
/// statement returns FAILURE.
///
/// This is equivalent to add AlwaysFailure as 3rd child.
#[bt_node(ControlNode)]
pub struct IfThenElseNode {
    #[bt(default = "0")]
    child_idx: usize,
}

impl AsyncTick for IfThenElseNode {
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

            if self.child_idx == 0 {
                let status = self.children[0].execute_tick().await?;
                match status {
                    NodeStatus::Running => return Ok(NodeStatus::Running),
                    NodeStatus::Success => self.child_idx += 1,
                    NodeStatus::Failure => {
                        if children_count == 3 {
                            self.child_idx = 2;
                        } else {
                            return Ok(NodeStatus::Failure);
                        }
                    }
                    NodeStatus::Idle => {
                        return Err(NodeError::StatusError(
                            "Node name here".to_string(),
                            "Idle".to_string(),
                        ))
                    }
                    _ => warn!("Condition node of IfThenElseNode returned Skipped"),
                }
            }

            if self.child_idx > 0 {
                let status = self.children[self.child_idx].execute_tick().await?;
                match status {
                    NodeStatus::Running => return Ok(NodeStatus::Running),
                    status => {
                        self.reset_children();
                        self.child_idx = 0;
                        return Ok(status);
                    }
                }
            }

            Err(NodeError::NodeStructureError(
                "Something unexpected happened in IfThenElseNode".to_string(),
            ))
        })
    }
}

impl NodePorts for IfThenElseNode {}

impl AsyncHalt for IfThenElseNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.child_idx = 0;
            self.reset_children();
        })
    }
}
