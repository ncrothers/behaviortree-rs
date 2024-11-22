use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::ControlNode;

/// The ReactiveFallback is similar to a ParallelNode.
/// All the children are ticked from first to last:
///
/// - If a child returns RUNNING, continue to the next sibling.
/// - If a child returns FAILURE, continue to the next sibling.
/// - If a child returns SUCCESS, stop and return SUCCESS.
///
/// If all the children fail, than this node returns FAILURE.
///
/// IMPORTANT: to work properly, this node should not have more than
///            a single asynchronous child.
#[derive(Debug)]
pub struct ReactiveFallbackNode;

impl ControlNode for ReactiveFallbackNode {
    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        let mut all_skipped = true;
        ctx.status = NodeStatus::Running;

        for index in 0..ctx.children.len() {
            let cur_child = &mut ctx.children[index];

            let child_status = cur_child.execute_tick()?;

            all_skipped &= child_status == NodeStatus::Skipped;

            match &child_status {
                NodeStatus::Running => {
                    for i in 0..index {
                        ctx.halt_child_idx(i)?;
                    }

                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {}
                NodeStatus::Success => {
                    ctx.reset_children()?;
                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Skipped => {
                    ctx.halt_child_idx(index)?;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "Name here".to_string(),
                        "Idle".to_string(),
                    ));
                }
            };
        }

        ctx.reset_children()?;

        match all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Failure),
        }
    }

    fn halt(&mut self, ctx: &mut NodeData) -> NodeResult<()> {
        ctx.reset_children()
    }
}
