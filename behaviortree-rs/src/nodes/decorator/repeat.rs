use crate::{
    basic_types::{NodeStatus, PortInfo},
    nodes::{NodeData, NodeError, NodeResult, PortsList},
};

use super::{DecoratorContext, DecoratorNode};

/// /// The RetryNode is used to execute a child several times, as long
/// as it succeed.
///
/// To succeed, the child must return SUCCESS N times (port "num_cycles").
///
/// If the child returns FAILURE, the loop is stopped and this node
/// returns FAILURE.
///
/// Example:
///
/// ```xml
/// <Repeat num_cycles="3">
///   <ClapYourHandsOnce/>
/// </Repeat>
/// ```
#[derive(Debug)]
pub struct RepeatNode {
    /// Default: -1
    num_cycles: i32,
    /// Default: 0
    repeat_count: usize,
}

impl Default for RepeatNode {
    fn default() -> Self {
        Self {
            num_cycles: -1,
            repeat_count: 0,
        }
    }
}

impl DecoratorNode for RepeatNode {
    fn ports(&self) -> crate::basic_types::PortsList {
        PortsList::from([PortInfo::input::<i32>("num_cycles").build()])
    }

    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        // Load num_cycles from the port value
        self.num_cycles = ctx.get_input("num_cycles")?;

        let mut do_loop = (self.repeat_count as i32) < self.num_cycles || self.num_cycles == -1;

        ctx.set_status(NodeStatus::Running);

        while do_loop {
            let child_status = ctx.child_mut().execute_tick()?;

            match child_status {
                NodeStatus::Success => {
                    self.repeat_count += 1;
                    do_loop = (self.repeat_count as i32) < self.num_cycles || self.num_cycles == -1;

                    ctx.reset_child()?;
                }
                NodeStatus::Failure => {
                    self.repeat_count = 0;
                    ctx.reset_child()?;

                    return Ok(NodeStatus::Failure);
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

        self.repeat_count = 0;

        Ok(NodeStatus::Success)
    }

    fn halt(&mut self, _ctx: &mut NodeData<DecoratorContext>) -> NodeResult<()> {
        self.repeat_count = 0;
        Ok(())
    }
}
