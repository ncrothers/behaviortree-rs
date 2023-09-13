use bt_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, TreeNode, NodeError, NodeHalt},
};

/// The ForceFailureNode returns always Failure or Running
#[bt_node(DecoratorNode)]
pub struct ForceFailureNode {}

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