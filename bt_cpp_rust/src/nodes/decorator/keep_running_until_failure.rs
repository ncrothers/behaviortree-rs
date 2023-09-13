use bt_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, TreeNode, NodeError, NodeHalt},
};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[bt_node(DecoratorNode)]
pub struct KeepRunningUntilFailureNode {}

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