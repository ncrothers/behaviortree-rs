use behaviortree_rs_derive::bt_node;

use crate::{
    basic_types::NodeStatus,
    nodes::{NodeError, NodeResult},
};

/// The ReactiveSequence is similar to a ParallelNode.
/// All the children are ticked from first to last:
///
/// - If a child returns RUNNING, halt the remaining siblings in the sequence and return RUNNING.
/// - If a child returns SUCCESS, tick the next sibling.
/// - If a child returns FAILURE, stop and return FAILURE.
///
/// If all the children return SUCCESS, this node returns SUCCESS.
///
/// IMPORTANT: to work properly, this node should not have more than a single
///            asynchronous child.
#[bt_node(ControlNode)]
pub struct ReactiveSequenceNode {
    #[bt(default = "-1")]
    running_child: i32,
}

#[bt_node(ControlNode)]
impl ReactiveSequenceNode {
    async fn tick(&mut self) -> NodeResult {
        let mut all_skipped = true;

        node_.status = NodeStatus::Running;

        for counter in 0..node_.children.len() {
            let child = &mut node_.children[counter];
            let child_status = child.execute_tick().await?;

            all_skipped &= child_status == NodeStatus::Skipped;

            match child_status {
                NodeStatus::Running => {
                    for i in 0..counter {
                        node_.children[i].halt().await;
                        // node_.halt_child(i).await?;
                    }
                    if self.running_child == -1 {
                        self.running_child = counter as i32;
                    } else if self.running_child != counter as i32 {
                        // Multiple children running at the same time
                        return Err(NodeError::NodeStructureError(
                            "[ReactiveSequence]: Only a single child can return Running."
                                .to_string(),
                        ));
                    }
                    return Ok(NodeStatus::Running);
                }
                NodeStatus::Failure => {
                    node_.reset_children().await;
                    return Ok(NodeStatus::Failure);
                }
                // Do nothing on Success
                NodeStatus::Success => {}
                NodeStatus::Skipped => {
                    // Halt current child
                    node_.children[counter].halt().await;
                    // node_.halt_child(counter).await?;
                }
                NodeStatus::Idle => {
                    return Err(NodeError::StatusError(
                        "ReactiveSequenceNode".into(),
                        "Idle".to_string(),
                    ));
                }
            }
        }

        for child in node_.children.iter_mut() {
            child.halt().await;
        }

        match all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Success),
        }
    }

    async fn halt(&mut self) {
        node_.reset_children().await;
    }
}
