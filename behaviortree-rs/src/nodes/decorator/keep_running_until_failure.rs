use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeResult},
};

use super::{Decorator, DecoratorNode};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[derive(Debug, Default)]
pub struct KeepRunningUntilFailureNode;

impl DecoratorNode for KeepRunningUntilFailureNode {
    fn tick(&mut self, ctx: &mut NodeData<Decorator>) -> NodeResult {
        ctx.set_status(NodeStatus::Running);

        let child_status = ctx.child().unwrap().execute_tick()?;

        match child_status {
            NodeStatus::Success => {
                ctx.reset_child()?;
                Ok(NodeStatus::Running)
            }
            NodeStatus::Failure => {
                ctx.reset_child()?;
                Ok(NodeStatus::Failure)
            }
            _ => Ok(NodeStatus::Running),
        }
    }

    fn halt(&mut self, ctx: &mut NodeData<Decorator>) -> NodeResult<()> {
        ctx.reset_child()
    }
}
