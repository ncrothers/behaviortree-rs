use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

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
#[derive(Debug, Default)]
pub struct IfThenElseNode {
    /// Default: 0
    child_idx: usize,
}

impl ControlNode for IfThenElseNode {
    type Context = ControlContext;

    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        let children_count = ctx.children.len();
        // Node should only have 2 or 3 children
        if !(2..=3).contains(&children_count) {
            return Err(NodeError::NodeStructureError(
                "IfThenElseNode must have either 2 or 3 children.".to_string(),
            ));
        }

        ctx.set_status(NodeStatus::Running);

        if self.child_idx == 0 {
            let status = ctx.children[0].execute_tick()?;
            match status {
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Success => self.child_idx = 1,
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
                _ => log::warn!("Condition node of IfThenElseNode returned Skipped"),
            }
        }

        if self.child_idx > 0 {
            let status = ctx.children[self.child_idx].execute_tick()?;
            match status {
                NodeStatus::Running => return Ok(NodeStatus::Running),
                status => {
                    ctx.reset_children();
                    self.child_idx = 0;
                    return Ok(status);
                }
            }
        }

        Err(NodeError::NodeStructureError(
            "Something unexpected happened in IfThenElseNode".to_string(),
        ))
    }

    fn halt(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        self.child_idx = 0;
        ctx.reset_children();

        Ok(())
    }
}
