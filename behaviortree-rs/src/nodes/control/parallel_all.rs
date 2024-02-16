use std::collections::HashSet;

use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{ControlNode, NodeError, NodeResult, TreeNodeDefaults},
};

/// The ParallelAllNode execute all its children
/// __concurrently__, but not in separate threads!
///
/// It differs in the way ParallelNode works because the latter may stop
/// and halt other children if a certain number of SUCCESS/FAILURES is reached,
/// whilst this one will always complete the execution of ALL its children.
///
/// Note that threshold indexes work as in Python:
/// https://www.i2tutorials.com/what-are-negative-indexes-and-why-are-they-used/
///
/// Therefore -1 is equivalent to the number of children.
#[bt_node(
    node_type = ControlNode,
)]
pub struct ParallelAllNode {
    #[bt(default = "-1")]
    failure_threshold: i32,
    #[bt(default)]
    completed_list: HashSet<usize>,
    #[bt(default = "0")]
    failure_count: usize,
}

#[bt_node(
    node_type = ControlNode,
    ports = provided_ports,
    tick = tick,
    halt = halt,
)]
impl ParallelAllNode {
    fn failure_threshold(&self) -> usize {
        if self.failure_threshold < 0 {
            ((node_.children.len() as i32) + self.failure_threshold + 1).max(0) as usize
        } else {
            self.failure_threshold as usize
        }
    }

    async fn tick(&mut self) -> NodeResult {
        self.failure_threshold = node_.config_mut().get_input("max_failures")?;

        let children_count = node_.children.len();

        if (children_count as i32) < self.failure_threshold {
            return Err(NodeError::NodeStructureError(
                "Number of children is less than the threshold. Can never fail.".to_string(),
            ));
        }

        let mut skipped_count = 0;

        for i in 0..children_count {
            // Skip completed node
            if self.completed_list.contains(&i) {
                continue;
            }

            let status = node_.children[i].execute_tick().await?;
            match status {
                NodeStatus::Success => {
                    self.completed_list.insert(i);
                }
                NodeStatus::Failure => {
                    self.completed_list.insert(i);
                    self.failure_count += 1;
                }
                NodeStatus::Skipped => skipped_count += 1,
                NodeStatus::Running => {}
                // Throw error, should never happen
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "ParallelAllNode".to_string(),
                        "Idle".to_string(),
                    ))
                }
            }
        }

        if skipped_count == children_count {
            return Ok(NodeStatus::Skipped);
        }

        if skipped_count + self.completed_list.len() >= children_count {
            // Done!
            node_.reset_children().await;
            self.completed_list.clear();

            let status = if self.failure_count >= self.failure_threshold() {
                NodeStatus::Failure
            } else {
                NodeStatus::Success
            };

            // Reset failure_count after using it
            self.failure_count = 0;

            return Ok(status);
        }

        Ok(NodeStatus::Running)
    }

    fn provided_ports() -> crate::basic_types::PortsList {
        define_ports!(input_port!("max_failures", 1))
    }

    async fn halt(&mut self) {
        node_.reset_children().await;
    }
}
