use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

/// The ReactiveSequence is similar to a ParallelNode.
/// All the children are ticked from first to last:
///
/// - If a child returns RUNNING, halt the remaining siblings in the sequence and return RUNNING.
/// - If a child returns SUCCESS, tick the next sibling.
/// - If a child returns FAILURE, stop and return FAILURE.
///
/// If all the children return SUCCESS, this node returns SUCCESS.
///
/// IMPORTANT: to work properly, this node should not have more than a single
///            asynchronous child.
#[derive(Debug)]
pub struct ReactiveSequenceNode {
    /// Default: -1
    running_child: i32,
}

impl Default for ReactiveSequenceNode {
    fn default() -> Self {
        Self { running_child: -1 }
    }
}

impl ControlNode for ReactiveSequenceNode {
    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        let mut all_skipped = true;

        ctx.set_status(NodeStatus::Running);

        for index in 0..ctx.children.len() {
            let child = &mut ctx.children[index];
            let child_status = child.execute_tick()?;

            all_skipped &= child_status == NodeStatus::Skipped;

            match child_status {
                NodeStatus::Running => {
                    for i in 0..ctx.children.len() {
                        if i != index {
                            ctx.halt_child(i)?;
                        }
                    }
                    if self.running_child == -1 {
                        self.running_child = index as i32;
                    } else if self.running_child != index as i32 {
                        // Multiple children running at the same time
                        return Err(NodeError::NodeStructureError(
                            "[ReactiveSequence]: Only a single child can return Running."
                                .to_string(),
                        ));
                    }
                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {
                    ctx.reset_children()?;
                    return Ok(NodeStatus::Failure);
                }
                // Do nothing on Success
                NodeStatus::Success => {}
                NodeStatus::Skipped => {
                    // Halt current child
                    ctx.halt_child(index)?;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "ReactiveSequenceNode".into(),
                        "Idle".to_string(),
                    ));
                }
            }
        }

        self.halt(ctx)?;

        match all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Success),
        }
    }

    fn halt(&mut self, _ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        self.running_child = -1;
        Ok(())
    }
}
