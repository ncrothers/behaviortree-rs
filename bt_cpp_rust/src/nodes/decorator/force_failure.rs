use bt_derive::{DecoratorNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
};

/// The ForceFailureNode returns always Failure or Running
#[derive(TreeNodeDefaults, DecoratorNode, Debug, Clone)]
pub struct ForceFailureNode {
    config: NodeConfig,
    child: Option<TreeNodePtr>,
    status: NodeStatus,
}

impl ForceFailureNode {
    pub fn new(config: NodeConfig) -> ForceFailureNode {
        Self {
            config,
            child: None,
            status: NodeStatus::Idle,
        }
    }
}

impl TreeNode for ForceFailureNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        self.set_status(NodeStatus::Running);

        let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick()?;

        if child_status.is_completed() {
            self.reset_child();

            return Ok(NodeStatus::Failure);
        }

        Ok(child_status)
    }
}

impl NodeHalt for ForceFailureNode {
    fn halt(&mut self) {
        self.reset_child();
    }
}