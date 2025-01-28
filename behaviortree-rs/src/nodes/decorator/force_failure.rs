use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeResult},
};

use super::{DecoratorContext, DecoratorNode};

/// The ForceFailureNode returns always Failure or Running
#[derive(Debug, Default)]
pub struct ForceFailureNode;

impl DecoratorNode for ForceFailureNode {
    type Context = ();

    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        ctx.set_status(NodeStatus::Running);

        let child_status = ctx.child().execute_tick()?;

        if child_status.is_completed() {
            ctx.reset_child()?;

            return Ok(NodeStatus::Failure);
        }

        Ok(child_status)
    }
}
