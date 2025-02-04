use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

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
pub struct ReactiveFallbackNode {
    /// Default: -1
    running_child: i32,
}

impl Default for ReactiveFallbackNode {
    fn default() -> Self {
        Self { running_child: -1 }
    }
}

impl ControlNode for ReactiveFallbackNode {
    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        let mut all_skipped = true;
        ctx.set_status(NodeStatus::Running);

        for index in 0..ctx.children.len() {
            let cur_child = &mut ctx.children[index];

            let child_status = cur_child.execute_tick()?;

            all_skipped &= child_status == NodeStatus::Skipped;

            match &child_status {
                NodeStatus::Running => {
                    for i in 0..ctx.children.len() {
                        if i != index {
                            ctx.halt_child(i)?;
                        }
                    }

                    // Check if there are two running children
                    if self.running_child == -1 {
                        self.running_child = index as i32;
                    } else if self.running_child != index as i32 {
                        // Multiple children running at the same time
                        return Err(NodeError::NodeStructureError(
                            "[ReactiveFallback]: Only a single child can return Running."
                                .to_string(),
                        ));
                    }

                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {}
                NodeStatus::Success => {
                    ctx.reset_children()?;
                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Skipped => {
                    ctx.halt_child(index)?;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "ReactiveFallback".to_string(),
                        "Idle".to_string(),
                    ));
                }
            };
        }

        self.halt(ctx)?;

        match all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Failure),
        }
    }

    fn halt(&mut self, _ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        self.running_child = -1;
        Ok(())
    }
}
