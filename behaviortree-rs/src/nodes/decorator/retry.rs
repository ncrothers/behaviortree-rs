use crate::{
    basic_types::{NodeStatus, PortInfo},
    nodes::{NodeData, NodeError, NodeResult, PortsList},
};

use super::{DecoratorContext, DecoratorNode};

/// The RetryNode is used to execute a child several times if it fails.
///
/// If the child returns SUCCESS, the loop is stopped and this node
/// returns SUCCESS.
///
/// If the child returns FAILURE, this node will try again up to N times
/// (N is read from port "num_attempts").
///
/// Example:
///
/// ```xml
/// <RetryUntilSuccessful num_attempts="3">
///     <OpenDoor/>
/// </RetryUntilSuccessful>
/// ```
#[derive(Debug)]
pub struct RetryNode {
    /// Default: -1
    max_attempts: i32,
    /// Default: 0
    try_count: usize,
}

impl Default for RetryNode {
    fn default() -> Self {
        Self {
            max_attempts: -1,
            try_count: 0,
        }
    }
}

impl DecoratorNode for RetryNode {
    fn ports(&self) -> crate::basic_types::PortsList {
        PortsList::from([PortInfo::input::<i32>("num_attempts")
            .default_value(1)
            .build()])
    }

    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        // Load num_cycles from the port value
        self.max_attempts = ctx.get_input("num_attempts")?;

        let mut do_loop = (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;

        ctx.set_status(NodeStatus::Running);

        while do_loop {
            let child_status = ctx.child_mut().execute_tick()?;

            match child_status {
                NodeStatus::Success => {
                    self.try_count = 0;
                    ctx.reset_child()?;

                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Failure => {
                    self.try_count += 1;
                    do_loop =
                        (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;

                    ctx.reset_child()?;
                }
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Skipped => {
                    ctx.reset_child()?;

                    return Ok(NodeStatus::Skipped);
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "InverterNode".to_string(),
                        "Idle".to_string(),
                    ))
                }
            }
        }

        self.try_count = 0;

        Ok(NodeStatus::Failure)
    }

    fn halt(&mut self, _ctx: &mut NodeData<DecoratorContext>) -> NodeResult<()> {
        self.try_count = 0;
        Ok(())
    }
}
