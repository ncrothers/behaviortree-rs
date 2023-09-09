use bt_derive::{DecoratorNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[derive(TreeNodeDefaults, DecoratorNode, Debug, Clone)]
pub struct KeepRunningUntilFailureNode {
    config: NodeConfig,
    child: Option<TreeNodePtr>,
    status: NodeStatus,
}

impl KeepRunningUntilFailureNode {
    pub fn new(config: NodeConfig) -> KeepRunningUntilFailureNode {
        Self {
            config,
            child: None,
            status: NodeStatus::Idle,
        }
    }
}

impl TreeNode for KeepRunningUntilFailureNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        self.set_status(NodeStatus::Running);

        let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick()?;

        match child_status {
            NodeStatus::Success => {
                self.reset_child();
                Ok(NodeStatus::Running)
            }
            NodeStatus::Failure => {
                self.reset_child();
                Ok(NodeStatus::Failure)
            }
            _ => Ok(NodeStatus::Running)
        }
    }
}

impl NodeHalt for KeepRunningUntilFailureNode {
    fn halt(&mut self) {
        self.reset_child();
    }
}