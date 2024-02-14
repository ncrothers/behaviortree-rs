use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{
        AsyncHalt, AsyncTick, DecoratorNode, NodeError, NodePorts, NodeResult, TreeNodeDefaults,
    },
};

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
#[bt_node(DecoratorNode)]
pub struct RetryNode {
    #[bt(default = "-1")]
    max_attempts: i32,
    #[bt(default = "0")]
    try_count: usize,
    #[bt(default = "true")]
    all_skipped: bool,
}

impl AsyncTick for RetryNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            // Load num_cycles from the port value
            self.max_attempts = self.config.get_input("num_attempts")?;

            let mut do_loop =
                (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;

            if matches!(self.status, NodeStatus::Idle) {
                self.all_skipped = true;
            }

            self.set_status(NodeStatus::Running);

            while do_loop {
                let child_status = self.child.as_mut().unwrap().execute_tick().await?;

                self.all_skipped &= matches!(child_status, NodeStatus::Skipped);

                match child_status {
                    NodeStatus::Success => {
                        self.try_count = 0;
                        self.reset_child().await;

                        return Ok(NodeStatus::Success);
                    }
                    NodeStatus::Failure => {
                        self.try_count += 1;
                        do_loop =
                            (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;

                        self.reset_child().await;
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

            self.try_count = 0;

            match self.all_skipped {
                true => Ok(NodeStatus::Skipped),
                false => Ok(NodeStatus::Failure),
            }
        })
    }
}

impl NodePorts for RetryNode {
    fn provided_ports(&self) -> crate::basic_types::PortsList {
        define_ports!(input_port!("num_attempts"))
    }
}

impl AsyncHalt for RetryNode {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.try_count = 0;
            self.reset_child().await;
        })
    }
}
