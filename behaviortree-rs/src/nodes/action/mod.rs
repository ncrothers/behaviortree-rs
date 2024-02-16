use std::any::Any;

use super::{HaltFn, NodeConfig, NodeStatus, PortsFn, TickFn};

// pub trait ActionNodeBase: TreeNodeBase + ActionNode {}

// pub trait ActionNode {
//     fn execute_action_tick(&mut self) -> BoxFuture<NodeResult>;
// }

// pub trait SyncActionNode {}

// pub type ActionNodePtr = Rc<RefCell<dyn ActionNodeBase>>;

// pub trait AsyncStatefulActionNode {
//     fn on_start(&mut self) -> BoxFuture<NodeResult>;
//     fn on_running(&mut self) -> BoxFuture<NodeResult>;
//     fn on_halted(&mut self) -> BoxFuture<()> {
//         Box::pin(async move {})
//     }
// }

// pub trait SyncStatefulActionNode {
//     fn on_start(&mut self) -> NodeResult;
//     fn on_running(&mut self) -> NodeResult;
//     fn on_halted(&mut self) {}
// }

#[derive(Debug)]
pub enum ActionSubType {
    Sync,
    Stateful,
}

#[derive(Debug)]
pub struct ActionNode {
    name: String,
    type_str: String,
    subtype: ActionSubType,
    config: NodeConfig,
    status: NodeStatus,
    /// Function pointer to tick (on_running for stateful nodes)
    tick_fn: TickFn<ActionNode>,
    /// Function pointer to on_start function, if it exists
    start_fn: TickFn<ActionNode>,
    /// Function pointer to halt
    halt_fn: HaltFn<ActionNode>,
    ports_fn: PortsFn,
    context: Box<dyn Any + Send>,
}
