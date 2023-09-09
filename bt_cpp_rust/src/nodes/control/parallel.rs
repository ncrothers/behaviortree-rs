use std::collections::HashSet;

use bt_derive::{ControlNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{ControlNode, NodeConfig, TreeNode, TreeNodeDefaults, TreeNodePtr, NodeError, NodeHalt},
};
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
#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct ParallelNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
    status: NodeStatus,
    success_threshold: i32,
    failure_threshold: i32,
    completed_list: HashSet<usize>,
    success_count: usize,
    failure_count: usize,
}

impl ParallelNode {
    pub fn new(config: NodeConfig) -> ParallelNode {
        Self {
            config,
            children: Vec::new(),
            status: NodeStatus::Idle,
            success_threshold: -1,
            failure_threshold: -1,
            completed_list: HashSet::new(),
            success_count: 0,
            failure_count: 0,
        }
    }

    fn success_threshold(&self) -> usize {
        if self.success_threshold < 0 {
            ((self.children.len() as i32) + self.success_threshold + 1).max(0) as usize
        } else {
            self.success_threshold as usize
        }
    }

    fn failure_threshold(&self) -> usize {
        if self.failure_threshold < 0 {
            ((self.children.len() as i32) + self.failure_threshold + 1).max(0) as usize
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

impl TreeNode for ParallelNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        self.success_threshold = self.config().get_input("success_count").unwrap();
        self.failure_threshold = self.config().get_input("failure_count").unwrap();

        let children_count = self.children.len();

        if children_count < self.success_threshold() {
            return Err(NodeError::NodeStructureError("Number of children is less than the threshold. Can never succeed.".to_string()));
        }

        if children_count < self.failure_threshold() {
            return Err(NodeError::NodeStructureError("Number of children is less than the threshold. Can never fail.".to_string()));
        }

        let mut skipped_count = 0;

        for i in 0..children_count {
            if !self.completed_list.contains(&i) {
                let mut child = self.children[i].borrow_mut();
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

            let required_success_count = self.success_threshold();

            // Check if success condition has been met
            if self.success_count >= required_success_count
                || (self.success_threshold < 0
                    && (self.success_count + skipped_count) >= required_success_count)
            {
                self.clear();
                self.reset_children();
                return Ok(NodeStatus::Success);
            }

            if (children_count - self.failure_count) < required_success_count
                || self.failure_count == self.failure_threshold()
            {
                self.clear();
                self.reset_children();
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

    fn provided_ports(&self) -> crate::basic_types::PortsList {
        define_ports!(
            input_port!("success_count", -1),
            input_port!("failure_count", 1)
        )
    }
}

impl NodeHalt for ParallelNode {
    fn halt(&mut self) {
        self.halt_control()
    }
}