use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

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
#[derive(Debug)]
pub struct WhileDoElseNode;

impl ControlNode for WhileDoElseNode {
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

        let condition_status = ctx.children[0].execute_tick()?;

        if matches!(condition_status, NodeStatus::Running) {
            return Ok(NodeStatus::Running);
        }

        let mut status = NodeStatus::Idle;

        match condition_status {
            NodeStatus::Success => {
                if children_count == 3 {
                    ctx.halt_child(2)?;
                }

                status = ctx.children[1].execute_tick()?;
            }
            NodeStatus::Failure => match children_count {
                3 => {
                    ctx.halt_child(1)?;
                    status = ctx.children[2].execute_tick()?;
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
                ctx.reset_children();
                Ok(status)
            }
        }
    }

    fn halt(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        ctx.reset_children();
        Ok(())
    }
}
