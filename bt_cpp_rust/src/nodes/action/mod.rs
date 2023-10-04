use std::{cell::RefCell, rc::Rc};

use futures::future::BoxFuture;

use crate::nodes::{NodeResult, TreeNodeBase};

pub trait ActionNodeBase: TreeNodeBase + ActionNode {}

pub trait ActionNode {
    /// Creates a cloned version of itself as a `ActionNode` trait object
    fn clone_boxed(&self) -> Box<dyn ActionNodeBase + Send + Sync>;
    fn execute_action_tick(&mut self) -> BoxFuture<NodeResult>;
}

impl Clone for Box<dyn ActionNodeBase + Send + Sync> {
    fn clone(&self) -> Box<dyn ActionNodeBase + Send + Sync> {
        self.clone_boxed()
    }
}

pub trait SyncActionNode {}

pub type ActionNodePtr = Rc<RefCell<dyn ActionNodeBase>>;

pub trait AsyncStatefulActionNode {
    fn on_start(&mut self) -> BoxFuture<NodeResult>;
    fn on_running(&mut self) -> BoxFuture<NodeResult>;
    fn on_halted(&mut self) -> BoxFuture<()> {
        Box::pin(async move {})
    }
}

pub trait SyncStatefulActionNode {
    fn on_start(&mut self) -> NodeResult;
    fn on_running(&mut self) -> NodeResult;
    fn on_halted(&mut self) {}
}
