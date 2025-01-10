use crate::nodes::{NodeData, NodeResult};

use super::{DecoratorContext, DecoratorNode};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[derive(Debug, Default)]
pub struct SubTreeNode;

impl DecoratorNode for SubTreeNode {
    type Context = DecoratorContext;

    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        let child_status = ctx.child().unwrap().execute_tick()?;

        ctx.set_status(child_status);

        Ok(child_status)
    }

    fn halt(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult<()> {
        ctx.reset_child()
    }
}
