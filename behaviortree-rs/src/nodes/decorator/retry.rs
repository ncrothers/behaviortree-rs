use behaviortree_rs_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{NodeError, NodeResult},
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

#[bt_node(
    node_type = DecoratorNode,
    ports = provided_ports,
    tick = tick,
    halt = halt,
)]
impl RetryNode {
    async fn tick(&mut self) -> NodeResult {
        // Load num_cycles from the port value
        self.max_attempts = node_.config.get_input("num_attempts")?;

        let mut do_loop = (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;

        if matches!(node_.status, NodeStatus::Idle) {
            self.all_skipped = true;
        }

        node_.status = NodeStatus::Running;

        while do_loop {
            let child_status = node_.child.as_mut().unwrap().execute_tick().await?;

            self.all_skipped &= matches!(child_status, NodeStatus::Skipped);

            match child_status {
                NodeStatus::Success => {
                    self.try_count = 0;
                    if let Some(child) = node_.child.as_mut() {
                        if matches!(child.status(), NodeStatus::Running) {
                            child.halt().await;
                        }

                        child.reset_status();
                    }

                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Failure => {
                    self.try_count += 1;
                    do_loop =
                        (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;

                    if let Some(child) = node_.child.as_mut() {
                        if matches!(child.status(), NodeStatus::Running) {
                            child.halt().await;
                        }

                        child.reset_status();
                    }
                }
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Skipped => {
                    if let Some(child) = node_.child.as_mut() {
                        if matches!(child.status(), NodeStatus::Running) {
                            child.halt().await;
                        }

                        child.reset_status();
                    }

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
    }

    fn provided_ports() -> crate::basic_types::PortsList {
        define_ports!(input_port!("num_attempts"))
    }

    async fn halt(&mut self) {
        self.try_count = 0;
        node_.reset_child().await;
    }
}
