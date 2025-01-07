use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeResult},
};

use super::{Decorator, DecoratorNode};

/// The ForceFailureNode returns always Failure or Running
#[derive(Debug, Default)]
pub struct ForceFailureNode;

impl DecoratorNode for ForceFailureNode {
    fn tick(&mut self, ctx: &mut NodeData<Decorator>) -> NodeResult {
        ctx.set_status(NodeStatus::Running);

        let child_status = ctx.child().unwrap().execute_tick()?;

        if child_status.is_completed() {
            ctx.reset_child()?;

            return Ok(NodeStatus::Failure);
        }

        Ok(child_status)
    }

    fn halt(&mut self, ctx: &mut NodeData<Decorator>) -> NodeResult<()> {
        ctx.reset_child()
    }
}
