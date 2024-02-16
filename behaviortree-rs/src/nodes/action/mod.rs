use std::any::Any;

use crate::{nodes::NodeError, NodeResult};

use super::{HaltFn, NodeConfig, NodeStatus, PortsFn, PortsList, TickFn};

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

impl ActionNode {
    pub async fn execute_tick(&mut self) -> NodeResult {
        let prev_status = self.status;

        let new_status = match prev_status {
            NodeStatus::Idle => {
                ::log::debug!("[behaviortree_rs]: {}::on_start()", &self.config.path);
                let new_status = (self.start_fn)(self).await?;
                if matches!(new_status, NodeStatus::Idle) {
                    return Err(NodeError::StatusError(format!("{}::on_start()", self.config.path), "Idle".to_string()))
                }
                new_status
            }
            NodeStatus::Running => {
                ::log::debug!("[behaviortree_rs]: {}::on_running()", &self.config.path);
                let new_status = (self.tick_fn)(self).await?;
                if matches!(new_status, NodeStatus::Idle) {
                    return Err(NodeError::StatusError(format!("{}::on_running()", self.config.path), "Idle".to_string()))
                }
                new_status
            }
            prev_status => prev_status
        };

        self.set_status(new_status);

        Ok(new_status)
    }

    pub async fn halt(&mut self) {
        (self.halt_fn)(self).await
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
