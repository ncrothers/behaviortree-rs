use std::any::Any;

use crate::{
    nodes::{HaltFn, NodeConfig, NodeStatus, PortsFn, PortsList, TickFn, TreeNode},
    NodeResult,
};

mod force_failure;
pub use force_failure::*;
mod force_success;
pub use force_success::*;
mod inverter;
pub use inverter::*;
mod keep_running_until_failure;
pub use keep_running_until_failure::*;
mod repeat;
pub use repeat::*;
mod retry;
pub use retry::*;
mod run_once;
pub use run_once::*;

#[derive(Debug)]
pub struct DecoratorNode {
    pub name: String,
    pub type_str: String,
    pub config: NodeConfig,
    pub status: NodeStatus,
    /// Child node
    pub child: Option<Box<TreeNode>>,
    /// Function pointer to tick
    pub tick_fn: TickFn<DecoratorNode>,
    /// Function pointer to halt
    pub halt_fn: HaltFn<DecoratorNode>,
    pub ports_fn: PortsFn,
    pub context: Box<dyn Any + Send>,
}

impl DecoratorNode {
    pub async fn execute_tick(&mut self) -> NodeResult {
        (self.tick_fn)(self).await
    }

    pub async fn halt(&mut self) {
        (self.halt_fn)(self).await
    }

    async fn halt_child(&mut self) {
        self.reset_child().await
    }

    async fn reset_child(&mut self) {
        if let Some(child) = self.child.as_mut() {
            if matches!(child.status(), NodeStatus::Running) {
                child.halt().await;
            }

            child.reset_status();
        }
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
