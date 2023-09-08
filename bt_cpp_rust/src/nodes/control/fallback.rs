use bt_derive::{ControlNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
};

/// The FallbackNode is used to try different strategies,
/// until one succeeds.
/// If any child returns RUNNING, previous children will NOT be ticked again.
///
/// - If all the children return FAILURE, this node returns FAILURE.
///
/// - If a child returns RUNNING, this node returns RUNNING.
///
/// - If a child returns SUCCESS, stop the loop and return SUCCESS.
#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct FallbackNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
    status: NodeStatus,
    child_idx: usize,
    all_skipped: bool,
}

impl FallbackNode {
    pub fn new(config: NodeConfig) -> FallbackNode {
        Self {
            config,
            children: Vec::new(),
            status: NodeStatus::Idle,
            child_idx: 0,
            all_skipped: true,
        }
    }
}

impl TreeNode for FallbackNode {
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
                NodeStatus::Running => {
                    return Ok(child_status);
                }
                NodeStatus::Failure => {
                    self.child_idx += 1;
                }
                NodeStatus::Success => {
                    self.reset_children();
                    self.child_idx = 0;
                    return Ok(child_status);
                }
                NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError("Name here".to_string(), "Idle".to_string()));
                }
            };
        }

        if self.child_idx == self.children.len() {
            self.reset_children();
            self.child_idx = 0;
        }

        Ok(NodeStatus::Success)
    }
}

impl NodeHalt for FallbackNode {
    fn halt(&mut self) {
        self.reset_children()
    }
}