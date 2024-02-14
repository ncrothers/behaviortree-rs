use behaviortree_rs_derive::bt_node;
use futures::future::BoxFuture;

use crate::{
    basic_types::NodeStatus,
    nodes::{AsyncHalt, AsyncTick, ControlNode, NodeError, NodePorts, NodeResult},
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
    tick = tick,
    halt = halt,
)]
pub struct FallbackNode {
    #[bt(default = "0")]
    child_idx: usize,
    #[bt(default = "true")]
    all_skipped: bool,
}

impl FallbackNode {
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
                    NodeStatus::Running => {
                        return Ok(NodeStatus::Running);
                    }
                    NodeStatus::Failure => {
                        self.child_idx += 1;
                    }
                    NodeStatus::Success => {
                        self.reset_children().await;
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

            if self.child_idx == self.children.len() {
                self.reset_children().await;
                self.child_idx = 0;
            }

            match self.all_skipped {
                true => Ok(NodeStatus::Skipped),
                false => Ok(NodeStatus::Failure),
            }
        })
    }

    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {
            self.child_idx = 0;
            self.reset_children().await;
        })
    }
}
