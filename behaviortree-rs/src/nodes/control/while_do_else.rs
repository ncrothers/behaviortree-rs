use behaviortree_rs_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{NodeError, NodeResult},
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

#[bt_node(ControlNode)]
impl WhileDoElseNode {
    async fn tick(&mut self) -> NodeResult {
        let children_count = node_.children.len();
        // Node should only have 2 or 3 children
        if !(2..=3).contains(&children_count) {
            return Err(NodeError::NodeStructureError(
                "IfThenElseNode must have either 2 or 3 children.".to_string(),
            ));
        }

        node_.status = NodeStatus::Running;

        let condition_status = node_.children[0].execute_tick().await?;

        if matches!(condition_status, NodeStatus::Running) {
            return Ok(NodeStatus::Running);
        }

        let mut status = NodeStatus::Idle;

        match condition_status {
            NodeStatus::Success => {
                if children_count == 3 {
                    node_.halt_child_idx(2).await?;
                }

                status = node_.children[1].execute_tick().await?;
            }
            NodeStatus::Failure => match children_count {
                3 => {
                    node_.halt_child_idx(1).await?;
                    status = node_.children[2].execute_tick().await?;
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
                node_.reset_children().await;
                Ok(status)
            }
        }
    }

    async fn halt(&mut self) {
        node_.reset_children().await;
    }
}
