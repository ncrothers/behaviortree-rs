use std::collections::HashSet;

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{ControlContext, ControlNode};

/// The ParallelNode execute all its children
/// __concurrently__, but not in separate threads!
///
/// Even if this may look similar to ReactiveSequence,
/// this Control Node is the __only__ one that can have
/// multiple children RUNNING at the same time.
///
/// The Node is completed either when the `success_count`
/// or `failure_count` number is reached (both configured using ports).
///
/// If any of the thresholds is reached, and other children are still running,
/// they will be halted.
///
/// Note that threshold indexes work as in Python:
/// https://www.i2tutorials.com/what-are-negative-indexes-and-why-are-they-used/
///
/// Therefore -1 is equivalent to the last child.
#[derive(Debug)]
pub struct ParallelNode {
    /// Default: 0
    success_threshold: usize,
    /// Default: 0
    failure_threshold: usize,
    completed_list: HashSet<usize>,
    /// Default: 0
    success_count: usize,
    /// Default: 0
    failure_count: usize,
}

#[allow(clippy::derivable_impls)]
impl Default for ParallelNode {
    fn default() -> Self {
        Self {
            success_threshold: 0,
            failure_threshold: 0,
            completed_list: HashSet::default(),
            success_count: 0,
            failure_count: 0,
        }
    }
}

impl ParallelNode {
    fn threshold(&self, threshold: i32, n_children: usize) -> usize {
        if threshold < 0 {
            i32::max(0, n_children as i32 + threshold + 1) as usize
        } else {
            threshold as usize
        }
    }

    fn clear(&mut self) {
        self.completed_list.clear();
        self.success_count = 0;
        self.failure_count = 0;
    }
}

impl ControlNode for ParallelNode {
    fn ports(&self) -> crate::basic_types::PortsList {
        define_ports!(
            input_port!("success_count", -1),
            input_port!("failure_count", 1)
        )
    }

    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        let children_count = ctx.children.len();

        self.success_threshold = self.threshold(ctx.get_input("success_count")?, children_count);
        self.failure_threshold = ctx.get_input("failure_count").unwrap();

        if children_count < self.success_threshold {
            return Err(NodeError::NodeStructureError(
                "Number of children is less than the threshold. Can never succeed.".to_string(),
            ));
        }

        if children_count < self.failure_threshold {
            return Err(NodeError::NodeStructureError(
                "Number of children is less than the threshold. Can never fail.".to_string(),
            ));
        }

        let mut skipped_count = 0;

        for i in 0..children_count {
            if !self.completed_list.contains(&i) {
                let child = &mut ctx.children[i];
                match child.execute_tick()? {
                    NodeStatus::Skipped => skipped_count += 1,
                    NodeStatus::Success => {
                        self.completed_list.insert(i);
                        self.success_count += 1;
                    }
                    NodeStatus::Failure => {
                        self.completed_list.insert(i);
                        self.failure_count += 1;
                    }
                    NodeStatus::Running => {}
                    // Throw error, should never happen
                    NodeStatus::Idle => {}
                }
            }

            let required_success_count = self.success_threshold;

            // Check if success condition has been met
            if self.success_count >= required_success_count
                // Changed from BehaviorTree.CPP to always include skipped_count,
                // not just when the threshold is negative (since that doesn't
                // seem to make sense)
                || (self.success_count + skipped_count) >= required_success_count
            {
                self.clear();
                ctx.reset_children()?;
                return Ok(NodeStatus::Success);
            }

            if (children_count - self.failure_count) < required_success_count
                || self.failure_count >= self.failure_threshold
            {
                self.clear();
                ctx.reset_children()?;
                return Ok(NodeStatus::Failure);
            }
        }

        // If all children were skipped, return Skipped
        // Otherwise return Running
        match skipped_count == children_count {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Running),
        }
    }
}
