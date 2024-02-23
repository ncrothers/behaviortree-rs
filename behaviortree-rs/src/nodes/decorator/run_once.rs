use behaviortree_rs_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::NodeResult,
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
#[bt_node(DecoratorNode)]
pub struct RunOnceNode {
    #[bt(default = "false")]
    already_ticked: bool,
    #[bt(default = "NodeStatus::Idle")]
    returned_status: NodeStatus,
}

#[bt_node(DecoratorNode)]
impl RunOnceNode {
    async fn tick(&mut self) -> NodeResult {
        let skip = node_.config.get_input("then_skip")?;

        if self.already_ticked {
            return if skip {
                Ok(NodeStatus::Skipped)
            } else {
                Ok(self.returned_status.clone())
            };
        }

        node_.status = NodeStatus::Running;

        let status = node_.child().unwrap().execute_tick().await?;

        if status.is_completed() {
            self.already_ticked = true;
            self.returned_status = status;
            node_.reset_child().await;
        }

        Ok(status)
    }

    fn ports() -> crate::basic_types::PortsList {
        define_ports!(input_port!("then_skip", true))
    }

    async fn halt(&mut self) {
        node_.reset_child().await;
    }
}
