use std::any::Any;

use crate::{
    nodes::{HaltFn, NodeConfig, NodeError, NodeStatus, PortsFn, PortsList, TickFn},
    NodeResult,
};

#[derive(Debug)]
pub enum ActionSubType {
    Sync,
    Stateful,
}

#[derive(Debug)]
pub struct ActionNode {
    pub name: String,
    pub type_str: String,
    pub subtype: ActionSubType,
    pub config: NodeConfig,
    pub status: NodeStatus,
    /// Function pointer to tick (on_running for stateful nodes)
    pub tick_fn: TickFn<ActionNode>,
    /// Function pointer to on_start function, if it exists
    pub start_fn: TickFn<ActionNode>,
    /// Function pointer to halt
    pub halt_fn: HaltFn<ActionNode>,
    pub ports_fn: PortsFn,
    pub context: Box<dyn Any + Send>,
}

impl ActionNode {
    pub async fn execute_tick(&mut self) -> NodeResult {
        match self.subtype {
            ActionSubType::Stateful => {
                let prev_status = self.status;

                let new_status = match prev_status {
                    NodeStatus::Idle => {
                        ::log::debug!("[behaviortree_rs]: {}::on_start()", &self.config.path);
                        let new_status = (self.start_fn)(self).await?;
                        if matches!(new_status, NodeStatus::Idle) {
                            return Err(NodeError::StatusError(
                                format!("{}::on_start()", self.config.path),
                                "Idle".to_string(),
                            ));
                        }
                        new_status
                    }
                    NodeStatus::Running => {
                        ::log::debug!("[behaviortree_rs]: {}::on_running()", &self.config.path);
                        let new_status = (self.tick_fn)(self).await?;
                        if matches!(new_status, NodeStatus::Idle) {
                            return Err(NodeError::StatusError(
                                format!("{}::on_running()", self.config.path),
                                "Idle".to_string(),
                            ));
                        }
                        new_status
                    }
                    prev_status => prev_status,
                };

                self.set_status(new_status);

                Ok(new_status)
            }
            ActionSubType::Sync => match (self.tick_fn)(self).await? {
                status @ (NodeStatus::Running | NodeStatus::Idle) => {
                    Err(::behaviortree_rs::nodes::NodeError::StatusError(
                        self.config.path.clone(),
                        status.to_string(),
                    ))
                }
                status => Ok(status),
            },
        }
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
