use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeResult},
};

use super::{DecoratorContext, DecoratorNode};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[derive(Debug, Default)]
pub struct KeepRunningUntilFailureNode;

impl DecoratorNode for KeepRunningUntilFailureNode {
    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        ctx.set_status(NodeStatus::Running);

        let child_status = ctx.child().execute_tick()?;

        match child_status {
            NodeStatus::Success => {
                ctx.reset_child()?;
                Ok(NodeStatus::Running)
            }
            NodeStatus::Failure => {
                ctx.reset_child()?;
                Ok(NodeStatus::Failure)
            }
            NodeStatus::Running => Ok(NodeStatus::Running),
            _ => Ok(ctx.status()),
        }
    }
}
