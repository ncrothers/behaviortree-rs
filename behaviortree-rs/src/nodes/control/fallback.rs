use behaviortree_rs_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{NodeError, NodeResult},
};

/// The FallbackNode is used to try different strategies,
/// until one succeeds.
/// If any child returns RUNNING, previous children will NOT be ticked again.
///
/// - If all the children return FAILURE, this node returns FAILURE.
///
/// - If a child returns RUNNING, this node returns RUNNING.
///
/// - If a child returns SUCCESS, stop the loop and return SUCCESS.
// #[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
#[bt_node(
    node_type = ControlNode,
)]
pub struct FallbackNode {
    #[bt(default = "0")]
    child_idx: usize,
    #[bt(default = "true")]
    all_skipped: bool,
}

#[bt_node(node_type = ControlNode, tick = tick, halt = halt)]
impl FallbackNode {
    async fn tick(&mut self) -> NodeResult {
        if node_.status == NodeStatus::Idle {
            self.all_skipped = true;
        }

        node_.status = NodeStatus::Running;

        while self.child_idx < node_.children.len() {
            let cur_child = &mut node_.children[self.child_idx];

            let _prev_status = cur_child.status();
            let child_status = cur_child.execute_tick().await?;

            self.all_skipped &= child_status == NodeStatus::Skipped;

            match &child_status {
                NodeStatus::Running => {
                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {
                    self.child_idx += 1;
                }
                NodeStatus::Success => {
                    for child in node_.children.iter_mut() {
                        child.halt().await;
                    }
                    // node_.reset_children().await;
                    self.child_idx = 0;
                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "Name here".to_string(),
                        "Idle".to_string(),
                    ));
                }
            };
        }

        if self.child_idx == node_.children.len() {
            for child in node_.children.iter_mut() {
                child.halt().await;
            }
            // node_.reset_children().await;
            self.child_idx = 0;
        }

        match self.all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Failure),
        }
    }

    async fn halt(&mut self) {
        self.child_idx = 0;
        node_.reset_children().await;
    }
}
