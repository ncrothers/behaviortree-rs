use bt_derive::{ControlNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
};
/// The SequenceStarNode is used to tick children in an ordered sequence.
/// If any child returns RUNNING, previous children are not ticked again.
/// 
/// - If all the children return SUCCESS, this node returns SUCCESS.
/// 
/// - If a child returns RUNNING, this node returns RUNNING.
///   Loop is NOT restarted, the same running child will be ticked again.
/// 
/// - If a child returns FAILURE, stop the loop and return FAILURE.
///   Loop is NOT restarted, the same running child will be ticked again.
#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct SequenceWithMemoryNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
    status: NodeStatus,
    child_idx: usize,
    all_skipped: bool,
}

impl SequenceWithMemoryNode {
    pub fn new(config: NodeConfig) -> SequenceWithMemoryNode {
        Self {
            config,
            children: Vec::new(),
            status: NodeStatus::Idle,
            child_idx: 0,
            all_skipped: false,
        }
    }
}

impl TreeNode for SequenceWithMemoryNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        if self.status == NodeStatus::Idle {
            self.all_skipped = true;
        }

        self.status = NodeStatus::Running;

        while self.child_idx < self.children.len() {
            let cur_child = &mut self.children[self.child_idx];

            let _prev_status = cur_child.borrow().status();
            let child_status = cur_child.borrow_mut().execute_tick()?;

            self.all_skipped &= child_status == NodeStatus::Skipped;

            match &child_status {
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Failure => {
                    // Do NOT reset child_idx on failure
                    // Halt children at and after this index
                    self.halt_children(self.child_idx)?;

                    return Ok(NodeStatus::Failure);
                }
                NodeStatus::Success | NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                NodeStatus::Idle => return Err(NodeError::StatusError("SequenceStarNode".to_string(), "Idle".to_string()))
            };
        }

        // All children returned Success
        if self.child_idx == self.children.len() {
            self.reset_children();
            self.child_idx = 0;
        }

        match self.all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Failure),
        }
    }
}

impl NodeHalt for SequenceWithMemoryNode {
    fn halt(&mut self) {
        self.child_idx = 0;
        self.reset_children()
    }
}