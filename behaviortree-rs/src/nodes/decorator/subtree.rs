use crate::nodes::{NodeData, NodeResult, NodeStatus};

use super::{DecoratorContext, DecoratorNode};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[derive(Debug, Default)]
pub struct SubTreeNode;

impl DecoratorNode for SubTreeNode {
    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        let prev_status = ctx.status();

        if matches!(prev_status, NodeStatus::Idle) {
            ctx.set_status(NodeStatus::Running);
        }

        let child_status = ctx.child().execute_tick()?;

        Ok(child_status)
    }
}
