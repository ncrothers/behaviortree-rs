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
/// The Node is completed either when the THRESHOLD_SUCCESS
/// or THRESHOLD_FAILURE number is reached (both configured using ports).
///
/// If any of the thresholds is reached, and other children are still running,
/// they will be halted.
///
/// Note that threshold indexes work as in Python:
/// https://www.i2tutorials.com/what-are-negative-indexes-and-why-are-they-used/
///
/// Therefore -1 is equivalent to the number of children.
#[derive(Debug)]
pub struct ParallelNode {
    /// Default: -1
    success_threshold: i32,
    /// Default: -1
    failure_threshold: i32,
    /// Defai;t
    completed_list: HashSet<usize>,
    /// Default: 0
    success_count: usize,
    /// Default: 0
    failure_count: usize,
}

impl Default for ParallelNode {
    fn default() -> Self {
        Self {
            success_threshold: -1,
            failure_threshold: -1,
            completed_list: HashSet::default(),
            success_count: 0,
            failure_count: 0,
        }
    }
}

impl ParallelNode {
    fn success_threshold(&self, n_children: i32) -> usize {
        if self.success_threshold < 0 {
            (n_children + self.success_threshold + 1).max(0) as usize
        } else {
            self.success_threshold as usize
        }
    }

    fn failure_threshold(&self, n_children: i32) -> usize {
        if self.failure_threshold < 0 {
            (n_children + self.failure_threshold + 1).max(0) as usize
        } else {
            self.failure_threshold as usize
        }
    }

    fn clear(&mut self) {
        self.completed_list.clear();
        self.success_count = 0;
        self.failure_count = 0;
    }
}

impl ControlNode for ParallelNode {
    type Context = ControlContext;

    fn ports(&self) -> crate::basic_types::PortsList {
        define_ports!(
            input_port!("success_count", -1),
            input_port!("failure_count", 1)
        )
    }

    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult {
        self.success_threshold = ctx.config.get_input("success_count").unwrap();
        self.failure_threshold = ctx.config.get_input("failure_count").unwrap();

        let children_count = ctx.children.len();

        if children_count < self.success_threshold(ctx.children.len() as i32) {
            return Err(NodeError::NodeStructureError(
                "Number of children is less than the threshold. Can never succeed.".to_string(),
            ));
        }

        if children_count < self.failure_threshold(ctx.children.len() as i32) {
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

            let required_success_count = self.success_threshold(ctx.children.len() as i32);

            // Check if success condition has been met
            if self.success_count >= required_success_count
                || (self.success_threshold < 0
                    && (self.success_count + skipped_count) >= required_success_count)
            {
                self.clear();
                ctx.reset_children();
                return Ok(NodeStatus::Success);
            }

            if (children_count - self.failure_count) < required_success_count
                || self.failure_count == self.failure_threshold(ctx.children.len() as i32)
            {
                self.clear();
                ctx.reset_children();
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

    fn halt(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        ctx.reset_children();
        Ok(())
    }
}
