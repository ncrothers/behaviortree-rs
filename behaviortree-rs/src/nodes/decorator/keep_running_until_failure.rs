use behaviortree_rs_derive::bt_node;

use crate::{basic_types::NodeStatus, nodes::NodeResult};

/// The KeepRunningUntilFailureNode returns always Failure or Running
#[bt_node(DecoratorNode)]
pub struct KeepRunningUntilFailureNode {}

#[bt_node(DecoratorNode)]
impl KeepRunningUntilFailureNode {
    async fn tick(&mut self) -> NodeResult {
        node_.set_status(NodeStatus::Running);

        let child_status = node_.child().unwrap().execute_tick().await?;

        match child_status {
            NodeStatus::Success => {
                node_.reset_child().await;
                Ok(NodeStatus::Running)
            }
            NodeStatus::Failure => {
                node_.reset_child().await;
                Ok(NodeStatus::Failure)
            }
            _ => Ok(NodeStatus::Running),
        }
    }

    async fn halt(&mut self) {
        node_.reset_child().await;
    }
}
