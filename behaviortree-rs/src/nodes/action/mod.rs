mod condition;
use std::ops::{Deref, DerefMut};

pub use condition::*;

use crate::nodes::NodeError;

use super::{NodeBase, NodeData, NodeDataGeneric, NodeResult, NodeStatus, PortsList, ToBoxed};

/// Empty marker struct to provide access to helper methods specific to SyncAction nodes
#[derive(Debug)]
pub struct SyncActionContext;

/// Wrapper struct around a boxed [`SyncActionNode`] implementer.
#[derive(Debug)]
pub struct SyncAction(Box<dyn SyncActionNode>);

/// Trait to implement for a node that is a "Sync Action Node", which just has
/// a `tick()` method when running, and is not allowed to return [`NodeStatus::Running`].
pub trait SyncActionNode: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult;

    #[allow(unused_variables)]
    fn halt(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult<()> {
        Ok(())
    }
}

impl Deref for SyncAction {
    type Target = Box<dyn SyncActionNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SyncAction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NodeBase for SyncAction {
    fn ports(&self) -> PortsList {
        self.0.ports()
    }

    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult {
        match self.0.tick(&mut NodeData::new(ctx))? {
            status @ (NodeStatus::Running | NodeStatus::Idle) => Err(NodeError::StatusError(
                ctx.meta.path.clone(),
                status.to_string(),
            )),
            status => Ok(status),
        }
    }

    fn halt(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult<()> {
        self.0.tick(&mut NodeData::new(ctx))?;
        ctx.set_status(NodeStatus::Idle);
        Ok(())
    }
}

impl<T> From<T> for SyncAction
where
    T: SyncActionNode + 'static,
{
    fn from(value: T) -> SyncAction {
        SyncAction(Box::new(value))
    }
}

impl<T> ToBoxed<SyncAction> for T
where
    T: SyncActionNode + 'static,
{
    fn to_boxed(self) -> Box<dyn NodeBase> {
        let node: SyncAction = self.into();
        Box::new(node)
    }
}

// =====================
// Stateful Action Node
// =====================

/// Empty marker struct to provide access to helper methods specific to StatefulAction nodes
#[derive(Debug)]
pub struct StatefulActionContext;

/// Wrapper struct around a boxed [`StatefulActionNode`] implementer.
#[derive(Debug)]
pub struct StatefulAction(Box<dyn StatefulActionNode>);

pub trait StatefulActionNode: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn on_start(&mut self, ctx: &mut NodeData<StatefulActionContext>) -> NodeResult;

    fn on_running(&mut self, ctx: &mut NodeData<StatefulActionContext>) -> NodeResult;

    #[allow(unused_variables)]
    fn on_halted(&mut self, ctx: &mut NodeData<StatefulActionContext>) -> NodeResult<()> {
        Ok(())
    }
}

impl Deref for StatefulAction {
    type Target = Box<dyn StatefulActionNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StatefulAction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NodeBase for StatefulAction {
    fn ports(&self) -> PortsList {
        self.0.ports()
    }

    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult {
        let prev_status = ctx.status();

        let new_status = match prev_status {
            NodeStatus::Idle => {
                ::log::debug!("[behaviortree_rs]: {}::on_start()", &ctx.meta.path);
                // let mut wrapper = ArgWrapper::new(&mut self.data, &mut self.context);
                let new_status = self.0.on_start(&mut NodeData::new(ctx))?;
                // drop(wrapper);
                if matches!(new_status, NodeStatus::Idle) {
                    return Err(NodeError::StatusError(
                        format!("{}::on_start()", ctx.meta.path),
                        "Idle".to_string(),
                    ));
                }
                new_status
            }
            NodeStatus::Running => {
                ::log::debug!("[behaviortree_rs]: {}::on_running()", &ctx.meta.path);
                let new_status = self.0.on_running(&mut NodeData::new(ctx))?;
                if matches!(new_status, NodeStatus::Idle) {
                    return Err(NodeError::StatusError(
                        format!("{}::on_running()", ctx.meta.path),
                        "Idle".to_string(),
                    ));
                }
                new_status
            }
            prev_status => prev_status,
        };

        ctx.set_status(new_status);

        Ok(new_status)
    }

    fn halt(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult<()> {
        self.0.on_halted(&mut NodeData::new(ctx))?;
        ctx.set_status(NodeStatus::Idle);
        Ok(())
    }
}

impl<T> From<T> for StatefulAction
where
    T: StatefulActionNode + 'static,
{
    fn from(value: T) -> StatefulAction {
        StatefulAction(Box::new(value))
    }
}

impl<T> ToBoxed<StatefulAction> for T
where
    T: StatefulActionNode + 'static,
{
    fn to_boxed(self) -> Box<dyn NodeBase> {
        let node: StatefulAction = self.into();
        Box::new(node)
    }
}
