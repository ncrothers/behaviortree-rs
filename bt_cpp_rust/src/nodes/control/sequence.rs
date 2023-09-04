use bt_derive::{ControlNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
};

/// The SequenceNode is used to tick children in an ordered sequence.
/// If any child returns RUNNING, previous children will NOT be ticked again.
/// 
/// - If all the children return SUCCESS, this node returns SUCCESS.
/// 
/// - If a child returns RUNNING, this node returns RUNNING.
///   Loop is NOT restarted, the same running child will be ticked again.
/// 
/// - If a child returns FAILURE, stop the loop and return FAILURE.
#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct SequenceNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
    status: NodeStatus,
    child_idx: usize,
    all_skipped: bool,
}

impl SequenceNode {
    pub fn new(config: NodeConfig) -> SequenceNode {
        Self {
            config,
            children: Vec::new(),
            status: NodeStatus::Idle,
            child_idx: 0,
            all_skipped: false,
        }
    }
}

impl TreeNode for SequenceNode {
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
                NodeStatus::Failure => {
                    self.reset_children();
                    self.child_idx = 0;
                    self.child_idx += 1;
                }
                NodeStatus::Success | NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                _ => {}
            };
        }

        if self.child_idx == self.children.len() {
            self.reset_children();
            self.child_idx = 0;
        }

        Ok(NodeStatus::Success)
    }
}

impl NodeHalt for SequenceNode {
    fn halt(&mut self) {
        self.reset_children()
    }
}