use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{
        AsyncHalt, AsyncTick, DecoratorNode, NodeError, NodePorts, NodeResult, TreeNodeDefaults,
    },
};

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
#[bt_node(DecoratorNode)]
pub struct RepeatNode {
    #[bt(default = "-1")]
    num_cycles: i32,
    #[bt(default = "0")]
    repeat_count: usize,
    #[bt(default = "true")]
    all_skipped: bool,
}

impl AsyncTick for RepeatNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            // Load num_cycles from the port value
            self.num_cycles = self.config.get_input("num_cycles").await?;

            let mut do_loop = (self.repeat_count as i32) < self.num_cycles || self.num_cycles == -1;

            if matches!(self.status, NodeStatus::Idle) {
                self.all_skipped = true;
            }

            self.set_status(NodeStatus::Running);

            while do_loop {
                let child_status = self
                    .child
                    .as_ref()
                    .unwrap()
                    .lock()
                    .await
                    .execute_tick()
                    .await?;

                self.all_skipped &= matches!(child_status, NodeStatus::Skipped);

                match child_status {
                    NodeStatus::Success => {
                        self.repeat_count += 1;
                        do_loop =
                            (self.repeat_count as i32) < self.num_cycles || self.num_cycles == -1;

                        self.reset_child().await;
                    }
                    NodeStatus::Failure => {
                        self.repeat_count = 0;
                        self.reset_child().await;

                        return Ok(NodeStatus::Failure);
                    }
                    NodeStatus::Running => return Ok(NodeStatus::Running),
                    NodeStatus::Skipped => {
                        self.reset_child().await;

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

            match self.all_skipped {
                true => Ok(NodeStatus::Skipped),
                false => Ok(NodeStatus::Success),
            }
        })
    }
}

impl NodePorts for RepeatNode {
    fn provided_ports(&self) -> crate::basic_types::PortsList {
        define_ports!(input_port!("num_cycles"))
    }
}

impl AsyncHalt for RepeatNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.repeat_count = 0;
            self.reset_child().await;
        })
    }
}
