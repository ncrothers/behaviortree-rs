use bt_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, TreeNode, NodeError, NodeHalt},
};

/// The InverterNode returns Failure on Success, and Success on Failure
#[bt_node(DecoratorNode)]
pub struct InverterNode {}

impl TreeNode for InverterNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        self.set_status(NodeStatus::Running);

        let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick()?;

        match child_status {
            NodeStatus::Success => {
                self.reset_child();
                Ok(NodeStatus::Failure)
            }
            NodeStatus::Failure => {
                self.reset_child();
                Ok(NodeStatus::Success)
            }
            status @ (NodeStatus::Running | NodeStatus::Skipped) => Ok(status),
            NodeStatus::Idle => Err(NodeError::StatusError("InverterNode".to_string(), "Idle".to_string())),
        }
    }
}

impl NodeHalt for InverterNode {
    fn halt(&mut self) {
        self.reset_child();
    }
}