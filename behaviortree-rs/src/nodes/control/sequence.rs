use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

/// The SequenceNode is used to tick children in an ordered sequence.
/// If any child returns RUNNING, previous children will NOT be ticked again.
///
/// - If all the children return SUCCESS, this node returns SUCCESS.
///
/// - If a child returns RUNNING, this node returns RUNNING.
///   Loop is NOT restarted, the same running child will be ticked again.
///
/// - If a child returns FAILURE, stop the loop and return FAILURE.
#[derive(Debug)]
pub struct SequenceNode {
    /// Default = 0
    child_idx: usize,
    /// Default: false
    all_skipped: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for SequenceNode {
    fn default() -> Self {
        Self {
            child_idx: 0,
            all_skipped: false,
        }
    }
}

impl ControlNode for SequenceNode {
    type Context = ();

    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        if ctx.status() == NodeStatus::Idle {
            self.all_skipped = true;
        }

        ctx.set_status(NodeStatus::Running);

        while self.child_idx < ctx.children.len() {
            let cur_child = &mut ctx.children[self.child_idx];

            let _prev_status = cur_child.status();
            let child_status = cur_child.execute_tick()?;

            self.all_skipped &= child_status == NodeStatus::Skipped;

            match &child_status {
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Failure => {
                    ctx.reset_children()?;
                    self.child_idx = 0;
                    return Ok(NodeStatus::Failure);
                }
                NodeStatus::Success | NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "SequenceNode".to_string(),
                        "Idle".to_string(),
                    ))
                }
            };
        }

        // Entire loop finished, meaning all children returned Success or Skipped
        if self.child_idx == ctx.children.len() {
            ctx.reset_children()?;
            self.child_idx = 0;
        }

        match self.all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Success),
        }
    }

    fn halt(&mut self, _ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        self.child_idx = 0;
        Ok(())
    }
}
