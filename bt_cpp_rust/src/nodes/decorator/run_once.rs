use bt_derive::{DecoratorNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
    macros::{define_ports, input_port}
};

/// The RunOnceNode is used when you want to execute the child
/// only once.
/// If the child is asynchronous, we will tick until either SUCCESS or FAILURE is
/// returned.
/// 
/// After that first execution, you can set value of the port "then_skip" to:
/// 
/// - if TRUE (default), the node will be skipped in the future.
/// - if FALSE, return synchronously the same status returned by the child, forever.
#[derive(TreeNodeDefaults, DecoratorNode, Debug, Clone)]
pub struct RunOnceNode {
    config: NodeConfig,
    child: Option<TreeNodePtr>,
    status: NodeStatus,
    already_ticked: bool,
    returned_status: NodeStatus,
}

impl RunOnceNode {
    pub fn new(config: NodeConfig) -> RunOnceNode {
        Self {
            config,
            child: None,
            status: NodeStatus::Idle,
            already_ticked: false,
            returned_status: NodeStatus::Idle,
        }
    }
}

impl TreeNode for RunOnceNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let skip = self.config.get_input("then_skip")?;

        if self.already_ticked {
            return if skip { Ok(NodeStatus::Skipped) } else { Ok(self.returned_status.clone()) };
        }

        self.set_status(NodeStatus::Running);

        let status = self.child.as_ref().unwrap().borrow_mut().execute_tick()?;

        if status.is_completed() {
            self.already_ticked = true;
            self.returned_status = status.clone();
            self.reset_child();
        }

        Ok(status)
    }

    fn provided_ports(&self) -> crate::basic_types::PortsList {
        define_ports!(
            input_port!("then_skip", true)
        )
    }
}

impl NodeHalt for RunOnceNode {
    fn halt(&mut self) {
        self.reset_child();
    }
}