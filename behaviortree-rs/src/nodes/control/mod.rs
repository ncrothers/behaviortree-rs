use std::any::Any;

use crate::{
    nodes::{HaltFn, NodeConfig, NodeError, NodeStatus, PortsFn, PortsList, TickFn, TreeNode},
    NodeResult,
};

mod if_then_else;
pub use if_then_else::*;
mod fallback;
pub use fallback::*;
mod reactive_fallback;
pub use reactive_fallback::*;
mod parallel;
pub use parallel::*;
mod parallel_all;
pub use parallel_all::*;
mod sequence;
pub use sequence::*;
mod sequence_star;
pub use sequence_star::*;
mod reactive_sequence;
pub use reactive_sequence::*;
mod while_do_else;
pub use while_do_else::*;

#[derive(Debug)]
pub struct ControlNode {
    pub name: String,
    pub type_str: String,
    pub config: NodeConfig,
    pub status: NodeStatus,
    /// Vector of child nodes
    pub children: Vec<TreeNode>,
    /// Function pointer to tick
    pub tick_fn: TickFn<ControlNode>,
    /// Function pointer to halt
    pub halt_fn: HaltFn<ControlNode>,
    pub ports_fn: PortsFn,
    pub context: Box<dyn Any + Send>,
}

impl ControlNode {
    pub async fn execute_tick(&mut self) -> NodeResult {
        (self.tick_fn)(self).await
    }

    pub async fn halt(&mut self) {
        (self.halt_fn)(self).await
    }

    pub async fn halt_child(&mut self, index: usize) -> NodeResult<()> {
        let child = self.children.get_mut(index).ok_or(NodeError::IndexError)?;
        if child.status() == ::behaviortree_rs::nodes::NodeStatus::Running {
            child.halt().await;
        }
        child.reset_status();
        Ok(())
    }

    pub async fn halt_children(&mut self, start: usize) -> NodeResult<()> {
        if start >= self.children.len() {
            return Err(NodeError::IndexError);
        }

        let end = self.children.len();

        for i in start..end {
            self.halt_child(i).await?;
        }

        Ok(())
    }

    pub async fn reset_children(&mut self) {
        self.halt_children(0)
            .await
            .expect("reset_children failed, shouldn't be possible. Report this")
    }

    pub fn config_mut(&mut self) -> &mut NodeConfig {
        &mut self.config
    }

    pub fn provided_ports(&self) -> PortsList {
        (self.ports_fn)()
    }

    pub fn set_status(&mut self, status: NodeStatus) {
        self.status = status;
    }
}
