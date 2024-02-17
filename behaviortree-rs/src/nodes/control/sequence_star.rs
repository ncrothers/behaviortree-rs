use behaviortree_rs_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{NodeError, NodeResult},
};
/// The SequenceStarNode is used to tick children in an ordered sequence.
/// If any child returns RUNNING, previous children are not ticked again.
///
/// - If all the children return SUCCESS, this node returns SUCCESS.
///
/// - If a child returns RUNNING, this node returns RUNNING.
///   Loop is NOT restarted, the same running child will be ticked again.
///
/// - If a child returns FAILURE, stop the loop and return FAILURE.
///   Loop is NOT restarted, the same running child will be ticked again.
#[bt_node(
    node_type = ControlNode,
)]
pub struct SequenceWithMemoryNode {
    #[bt(default = "0")]
    child_idx: usize,
    #[bt(default = "false")]
    all_skipped: bool,
}

#[bt_node(
    node_type = ControlNode,
    tick = tick,
    halt = halt,
)]
impl SequenceWithMemoryNode {
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
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Failure => {
                    // Do NOT reset child_idx on failure
                    // Halt children at and after this index
                    for i in self.child_idx..node_.children.len() {
                        node_.children[i].halt().await;
                    }
                    // node_.halt_children(self.child_idx).await?;

                    return Ok(NodeStatus::Failure);
                }
                NodeStatus::Success | NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "SequenceStarNode".to_string(),
                        "Idle".to_string(),
                    ))
                }
            };
        }

        // All children returned Success
        if self.child_idx == node_.children.len() {
            for child in node_.children.iter_mut() {
                child.halt().await;
            }
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
