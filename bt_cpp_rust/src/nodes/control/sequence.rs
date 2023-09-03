use bt_derive::{TreeNodeDefaults, ControlNode};

use crate::{nodes::{NodeConfig, TreeNodePtr, TreeNode, ControlNode}, basic_types::NodeStatus};

#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct SequenceNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
    status: NodeStatus,
    child_idx: usize,
    all_skipped: bool
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
    fn tick(&mut self) -> NodeStatus {
        if self.status == NodeStatus::Idle {
            self.all_skipped = true;
        }

        self.status = NodeStatus::Running;

        while self.child_idx < self.children.len() {
            let cur_child = &mut self.children[self.child_idx];

            let _prev_status = cur_child.borrow().status();
            let child_status = cur_child.borrow_mut().tick();

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

        NodeStatus::Success
    }

    fn halt(&mut self) {
        self.reset_children()
    }
}