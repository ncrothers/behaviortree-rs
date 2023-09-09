use crate::{nodes::{TreeNodeBase, NodeError}, basic_types::NodeStatus};

pub trait ActionNodeBase: TreeNodeBase + ActionNode {}

pub trait ActionNode {
    /// Creates a cloned version of itself as a `ActionNode` trait object
    fn clone_boxed(&self) -> Box<dyn ActionNodeBase>;
    fn execute_action_tick(&mut self) -> Result<NodeStatus, NodeError>;
}

impl Clone for Box<dyn ActionNodeBase> {
    fn clone(&self) -> Box<dyn ActionNodeBase> {
        self.clone_boxed()
    }
}

pub trait SyncActionNode {}

pub trait StatefulActionNode {
    fn on_start(&mut self) -> Result<NodeStatus, NodeError>;
    fn on_running(&mut self) -> Result<NodeStatus, NodeError>;
    fn on_halted(&mut self) {}
}