use bt_derive::{TreeNodeDefaults, ControlNode};
use log::error;

use crate::{nodes::{NodeConfig, TreeNodePtr, TreeNode, ControlNode}, basic_types::NodeStatus};

#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct ReactiveSequenceNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
    status: NodeStatus,
    running_child: i32
}

impl ReactiveSequenceNode {
    pub fn new(config: NodeConfig) -> ReactiveSequenceNode {
        Self {
            config,
            children: Vec::new(),
            status: NodeStatus::Idle,
            running_child: -1,
        }
    }
}

impl TreeNode for ReactiveSequenceNode {
    fn tick(&mut self) -> NodeStatus {
        let mut all_skipped = true;

        self.status = NodeStatus::Running;

        let mut counter: i32 = 0;

        for child in self.children.iter() {
            let child_status = child.borrow_mut().tick();

            all_skipped &= child_status == NodeStatus::Skipped;

            match child_status {
                NodeStatus::Running => {
                    for i in 0..counter {
                        if let Err(e) = self.halt_child(i as usize) {
                            error!("Unexpected error in ReactiveSequenceNode.tick(): {e:?}");
                        }
                    }
                    if self.running_child == -1 {
                        self.running_child = counter;
                    }
                    else if self.running_child != counter {
                        // Multiple children running at the same time
                    }
                    return NodeStatus::Running;
                }
                NodeStatus::Failure => {
                    self.reset_children();
                    return NodeStatus::Failure;
                }
                // Do nothing on Success
                NodeStatus::Success => {},
                NodeStatus::Skipped => {
                    // Halt current child
                    child.borrow_mut().halt();
                }
                NodeStatus::Idle => {
                    panic!("A child should never return NodeStatus::Idle");
                }
            }

            counter += 1;
        }

        self.reset_children();

        match all_skipped {
            true => NodeStatus::Skipped,
            false => NodeStatus::Success
        }
    }

    fn halt(&mut self) {
        self.reset_children()
    }
}