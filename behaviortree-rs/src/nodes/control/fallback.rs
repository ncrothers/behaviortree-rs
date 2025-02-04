use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

/// The FallbackNode is used to try different strategies,
/// until one succeeds.
/// If any child returns RUNNING, previous children will NOT be ticked again.
///
/// - If all the children return FAILURE, this node returns FAILURE.
///
/// - If a child returns RUNNING, this node returns RUNNING.
///
/// - If a child returns SUCCESS, stop the loop and return SUCCESS.
#[derive(Debug)]
pub struct FallbackNode {
    /// Default: 0
    child_idx: usize,
    /// Default: true
    all_skipped: bool,
}

impl Default for FallbackNode {
    fn default() -> Self {
        Self {
            child_idx: 0,
            all_skipped: true,
        }
    }
}

impl ControlNode for FallbackNode {
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
                NodeStatus::Running => {
                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {
                    self.child_idx += 1;
                }
                NodeStatus::Success => {
                    ctx.reset_children()?;
                    self.child_idx = 0;
                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "Name here".to_string(),
                        "Idle".to_string(),
                    ));
                }
            };
        }

        if self.child_idx == ctx.children.len() {
            ctx.reset_children()?;
            self.child_idx = 0;
        }

        match self.all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Failure),
        }
    }

    fn halt(&mut self, _ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        self.child_idx = 0;
        self.all_skipped = true;
        Ok(())
    }
}
