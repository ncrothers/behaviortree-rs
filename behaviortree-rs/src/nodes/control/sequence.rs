use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{ControlNode, NodeError, NodeResult},
};

/// The SequenceNode is used to tick children in an ordered sequence.
/// If any child returns RUNNING, previous children will NOT be ticked again.
///
/// - If all the children return SUCCESS, this node returns SUCCESS.
///
/// - If a child returns RUNNING, this node returns RUNNING.
///   Loop is NOT restarted, the same running child will be ticked again.
///
/// - If a child returns FAILURE, stop the loop and return FAILURE.
#[bt_node(
    node_type = ControlNode,
    tick = tick,
    halt = halt,
)]
pub struct SequenceNode {
    #[bt(default = "0")]
    child_idx: usize,
    #[bt(default = "false")]
    all_skipped: bool,
}

impl SequenceNode {
    fn tick(&mut self) -> BoxFuture<NodeResult> {
        Box::pin(async move {
            if self.status == NodeStatus::Idle {
                self.all_skipped = true;
            }

            self.status = NodeStatus::Running;

            while self.child_idx < self.children.len() {
                let cur_child = &mut self.children[self.child_idx];

                let _prev_status = cur_child.status();
                let child_status = cur_child.execute_tick().await?;

                self.all_skipped &= child_status == NodeStatus::Skipped;

                match &child_status {
                    NodeStatus::Failure => {
                        self.reset_children().await;
                        self.child_idx = 0;
                        return Ok(NodeStatus::Failure);
                    }
                    NodeStatus::Success | NodeStatus::Skipped => {
                        self.child_idx += 1;
                    }
                    NodeStatus::Idle => {
                        return Err(NodeError::StatusError(
                            "ParallelAllNode".to_string(),
                            "Idle".to_string(),
                        ))
                    }
                    _ => {}
                };
            }

            if self.child_idx == self.children.len() {
                self.reset_children().await;
                self.child_idx = 0;
            }

            Ok(NodeStatus::Success)
        })
    }

    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.child_idx = 0;
            self.reset_children().await;
        })
    }
}
