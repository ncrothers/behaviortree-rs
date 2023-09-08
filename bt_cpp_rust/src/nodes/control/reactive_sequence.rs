use bt_derive::{ControlNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
};

#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct ReactiveSequenceNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
    status: NodeStatus,
    running_child: i32,
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
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let mut all_skipped = true;

        self.status = NodeStatus::Running;

        for (counter, child) in self.children.iter().enumerate() {
            let child_status = child.borrow_mut().execute_tick()?;

            all_skipped &= child_status == NodeStatus::Skipped;

            match child_status {
                NodeStatus::Running => {
                    for i in 0..counter {
                        self.halt_child(i)?;
                        // if let Err(e) = self.halt_child(i as usize) {
                        //     error!("Unexpected error in ReactiveSequenceNode.tick(): {e:?}");
                        // }
                    }
                    if self.running_child == -1 {
                        self.running_child = counter as i32;
                    } else if self.running_child != counter as i32 {
                        // Multiple children running at the same time
                    }
                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {
                    self.reset_children();
                    return Ok(NodeStatus::Failure);
                }
                // Do nothing on Success
                NodeStatus::Success => {}
                NodeStatus::Skipped => {
                    // Halt current child
                    child.borrow_mut().halt();
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(child.borrow_mut().config().path.clone(), "Idle".to_string()));
                }
            }
        }

        self.reset_children();

        match all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Success),
        }
    }
}

impl NodeHalt for ReactiveSequenceNode {
    fn halt(&mut self) {
        self.reset_children()
    }
}