use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, NodeError, NodeResult},
};

/// The ReactiveFallback is similar to a ParallelNode.
/// All the children are ticked from first to last:
///
/// - If a child returns RUNNING, continue to the next sibling.
/// - If a child returns FAILURE, continue to the next sibling.
/// - If a child returns SUCCESS, stop and return SUCCESS.
///
/// If all the children fail, than this node returns FAILURE.
///
/// IMPORTANT: to work properly, this node should not have more than
///            a single asynchronous child.
#[bt_node(
    node_type = ControlNode,
)]
pub struct ReactiveFallbackNode {}

#[bt_node(
    node_type = ControlNode,
    tick = tick,
    halt = halt,
)]
impl ReactiveFallbackNode {
    async fn tick(&mut self) -> NodeResult {
        let mut all_skipped = true;
        node_.status = NodeStatus::Running;

        for index in 0..node_.children.len() {
            let cur_child = &mut node_.children[index];

            let child_status = cur_child.execute_tick().await?;

            all_skipped &= child_status == NodeStatus::Skipped;

            match &child_status {
                NodeStatus::Running => {
                    for i in 0..index {
                        node_.halt_child(i).await?;
                    }

                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {}
                NodeStatus::Success => {
                    node_.reset_children().await;
                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Skipped => {
                    node_.halt_child(index).await?;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "Name here".to_string(),
                        "Idle".to_string(),
                    ));
                }
            };
        }

        node_.reset_children().await;

        match all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Failure),
        }
    }

    async fn halt(&mut self) {
        node_.reset_children().await;
    }
}
