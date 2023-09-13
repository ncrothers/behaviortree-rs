use bt_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, TreeNode, NodeError, NodeHalt},
};

/// The ForceSuccessNode returns always Success or Running
#[bt_node(DecoratorNode)]
pub struct ForceSuccessNode {}

impl TreeNode for ForceSuccessNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        self.set_status(NodeStatus::Running);

        let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick()?;

        if child_status.is_completed() {
            self.reset_child();

            return Ok(NodeStatus::Success);
        }

        Ok(child_status)
    }
}

impl NodeHalt for ForceSuccessNode {
    fn halt(&mut self) {
        self.reset_child();
    }
}