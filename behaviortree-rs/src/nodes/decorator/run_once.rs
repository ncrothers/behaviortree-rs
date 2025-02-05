use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{NodeData, NodeResult},
};

use super::{DecoratorContext, DecoratorNode};

/// The RunOnceNode is used when you want to execute the child
/// only once.
/// If the child is asynchronous, we will tick until either SUCCESS or FAILURE is
/// returned.
///
/// After that first execution, you can set value of the port "then_skip" to:
///
/// - if TRUE (default), the node will be skipped in the future.
/// - if FALSE, return synchronously the same status returned by the child, forever.
#[derive(Debug)]
pub struct RunOnceNode {
    /// Default: false
    already_ticked: bool,
    /// Default: NodeStatus::Idle
    returned_status: NodeStatus,
}

impl Default for RunOnceNode {
    fn default() -> Self {
        Self {
            already_ticked: false,
            returned_status: NodeStatus::Idle,
        }
    }
}

impl DecoratorNode for RunOnceNode {
    fn ports(&self) -> crate::basic_types::PortsList {
        define_ports!(input_port!("then_skip", true))
    }

    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult {
        let skip = ctx.get_input("then_skip")?;

        if self.already_ticked {
            if skip {
                return Ok(NodeStatus::Skipped);
            } else {
                return Ok(self.returned_status);
            }
        }

        ctx.set_status(NodeStatus::Running);

        let status = ctx.child_mut().execute_tick()?;

        if status.is_completed() {
            self.already_ticked = true;
            self.returned_status = status;
            ctx.reset_child()?;
        }

        Ok(status)
    }
}
