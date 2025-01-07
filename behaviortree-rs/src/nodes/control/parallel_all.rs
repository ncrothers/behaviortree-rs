use std::collections::HashSet;

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

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
#[derive(Debug)]
pub struct ParallelAllNode {
    /// Default: -1
    failure_threshold: i32,
    /// Default: empty
    completed_list: HashSet<usize>,
    /// Default: 0
    failure_count: usize,
}

impl Default for ParallelAllNode {
    fn default() -> Self {
        Self {
            failure_threshold: -1,
            completed_list: HashSet::default(),
            failure_count: 0,
        }
    }
}

impl ParallelAllNode {
    fn failure_threshold(&self, n_children: i32) -> usize {
        if self.failure_threshold < 0 {
            (n_children + self.failure_threshold + 1).max(0) as usize
        } else {
            self.failure_threshold as usize
        }
    }
}

impl ControlNode for ParallelAllNode {
    type Context = ControlContext;

    fn ports(&self) -> crate::basic_types::PortsList {
        define_ports!(input_port!("max_failures", 1))
    }

    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        self.failure_threshold = ctx.config.get_input("max_failures")?;

        let children_count = ctx.children.len();

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

            let status = ctx.children[i].execute_tick()?;
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
            ctx.reset_children();
            self.completed_list.clear();

            let status = if self.failure_count >= self.failure_threshold(ctx.children.len() as i32)
            {
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

    fn halt(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        ctx.reset_children();
        Ok(())
    }
}
