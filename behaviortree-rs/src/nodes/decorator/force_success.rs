use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeResult},
};

use super::DecoratorNode;

/// The ForceSuccessNode returns always Success or Running
#[derive(Debug, Default)]
pub struct ForceSuccessNode;

impl DecoratorNode for ForceSuccessNode {
    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        ctx.set_status(NodeStatus::Running);

        let child_status = ctx.child().unwrap().execute_tick()?;

        if child_status.is_completed() {
            ctx.reset_child()?;

            return Ok(NodeStatus::Success);
        }

        Ok(child_status)
    }

    fn halt(&mut self, ctx: &mut NodeData) -> NodeResult<()> {
        ctx.reset_child()
    }
}
