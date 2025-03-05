use crate::{
    basic_types::{NodeStatus, PortInfo},
    nodes::{NodeData, NodeError, NodeResult, PortsList},
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
    /// Default: 0
    failure_threshold: usize,
    /// Default: empty
    completed_list: Vec<usize>,
    /// Default: 0
    failure_count: usize,
}

impl Default for ParallelAllNode {
    fn default() -> Self {
        Self {
            failure_threshold: 0,
            // Presize it to 10 so it's unlikely to need to grow
            completed_list: Vec::with_capacity(10),
            failure_count: 0,
        }
    }
}

impl ParallelAllNode {
    fn failure_threshold(&self, threshold: i32, n_children: usize) -> usize {
        if threshold < 0 {
            i32::max(0, n_children as i32 + threshold + 1) as usize
        } else {
            threshold as usize
        }
    }
}

impl ControlNode for ParallelAllNode {
    fn ports(&self) -> crate::basic_types::PortsList {
        PortsList::from([PortInfo::input::<i32>("max_failures")
            .default_value(1)
            .build()])
    }

    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        let children_count = ctx.children.len();

        self.failure_threshold =
            self.failure_threshold(ctx.get_input("max_failures")?, children_count);

        if children_count < self.failure_threshold {
            return Err(NodeError::NodeStructureError(
                "Number of children is less than the threshold. Can never fail.".to_string(),
            ));
        }

        let mut skipped_count = 0;

        ctx.set_status(NodeStatus::Running);

        for i in 0..children_count {
            // Skip completed node
            if self.completed_list.contains(&i) {
                continue;
            }

            let status = ctx.children[i].execute_tick()?;
            match status {
                NodeStatus::Success => {
                    self.completed_list.push(i);
                }
                NodeStatus::Failure => {
                    self.completed_list.push(i);
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
            ctx.reset_children()?;
            self.completed_list.clear();

            let status = if self.failure_count >= self.failure_threshold {
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
}
