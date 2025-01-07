use crate::nodes::{NodeData, NodeResult};

use super::{Decorator, DecoratorNode};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[derive(Debug, Default)]
pub struct SubTree;

impl DecoratorNode for SubTree {
    fn tick(&mut self, ctx: &mut NodeData<Decorator>) -> NodeResult {
        let child_status = ctx.child().unwrap().execute_tick()?;

        ctx.set_status(child_status);

        Ok(child_status)
    }

    fn halt(&mut self, ctx: &mut NodeData<Decorator>) -> NodeResult<()> {
        ctx.reset_child()
    }
}
