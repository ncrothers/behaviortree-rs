use crate::{
    basic_types::NodeStatus,
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{DecoratorContext, DecoratorNode};

/// The InverterNode returns Failure on Success, and Success on Failure
#[derive(Debug, Default)]
pub struct InverterNode;

impl DecoratorNode for InverterNode {
    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        ctx.set_status(NodeStatus::Running);

        let child_status = ctx.child_mut().execute_tick()?;

        match child_status {
            NodeStatus::Success => {
                ctx.reset_child()?;
                Ok(NodeStatus::Failure)
            }
            NodeStatus::Failure => {
                ctx.reset_child()?;
                Ok(NodeStatus::Success)
            }
            status @ (NodeStatus::Running | NodeStatus::Skipped) => Ok(status),
            NodeStatus::Idle => Err(NodeError::StatusError(
                "InverterNode".to_string(),
                "Idle".to_string(),
            )),
        }
    }
}
