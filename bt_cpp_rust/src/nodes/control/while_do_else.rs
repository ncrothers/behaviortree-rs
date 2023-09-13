use bt_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, TreeNode, TreeNodePtr, NodeError, NodeHalt},
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

impl TreeNode for WhileDoElseNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let children_count = self.children.len();
        // Node should only have 2 or 3 children
        if !(2..=3).contains(&children_count) {
            return Err(NodeError::NodeStructureError("IfThenElseNode must have either 2 or 3 children.".to_string()));
        }

        self.status = NodeStatus::Running;

        let condition_status = self.children[0].borrow_mut().execute_tick()?;

        if matches!(condition_status, NodeStatus::Running) {
            return Ok(NodeStatus::Running);
        }

        let mut status = NodeStatus::Idle;

        match condition_status {
            NodeStatus::Success => {
                if children_count == 3 {
                    self.halt_child(2)?;
                }

                status = self.children[1].borrow_mut().execute_tick()?;
            }
            NodeStatus::Failure => {
                match children_count {
                    3 => {
                        self.halt_child(1)?;
                        status = self.children[2].borrow_mut().execute_tick()?;
                    }
                    2 => {
                        status = NodeStatus::Failure;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        match status {
            NodeStatus::Running => Ok(NodeStatus::Running),
            status => {
                self.reset_children();
                Ok(status)
            }
        }
    }
}

impl NodeHalt for WhileDoElseNode {
    fn halt(&mut self) {
        self.reset_children()
    }
}