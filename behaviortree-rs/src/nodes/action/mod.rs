mod condition;
use std::ops::{Deref, DerefMut};

pub use condition::*;

use crate::nodes::NodeError;

use super::{NodeBase, NodeData, NodeDataGeneric, NodeResult, NodeStatus, PortsList, ToBoxed};

pub struct SyncActionContext<T = ()>(pub T);

impl<T> Deref for SyncActionContext<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for SyncActionContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Wrapper struct around a boxed [`SyncActionNode`] implementer.
#[derive(Debug)]
pub struct SyncAction(Box<dyn SyncActionNode<Context = SyncActionContext>>);

/// Trait to implement for a node that is a "Sync Action Node", which just has
/// a `tick()` method when running, and is not allowed to return [`NodeStatus::Running`].
pub trait SyncActionNode: std::fmt::Debug + Send + Sync {
    type Context;

    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn tick(&mut self, ctx: &mut NodeData<Self::Context>) -> NodeResult;

    #[allow(unused_variables)]
    fn halt(&mut self, ctx: &mut NodeData<Self::Context>) -> NodeResult<()> {
        Ok(())
    }
}

impl Deref for SyncAction {
    type Target = Box<dyn SyncActionNode<Context = SyncActionContext>>;

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
        SyncActionNode::ports(&*self.0)
    }

    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult {
        match self.tick(&mut NodeData::new(ctx, &mut SyncActionContext(())))? {
            status @ (NodeStatus::Running | NodeStatus::Idle) => Err(NodeError::StatusError(
                ctx.config.path.clone(),
                status.to_string(),
            )),
            status => Ok(status),
        }
    }

    fn halt(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult<()> {
        SyncActionNode::halt(
            &mut *self.0,
            &mut NodeData::new(ctx, &mut SyncActionContext(())),
        )
    }
}

impl<T> From<T> for SyncAction
where
    T: SyncActionNode<Context = SyncActionContext> + 'static,
{
    fn from(value: T) -> SyncAction {
        SyncAction(Box::new(value))
    }
}

impl<T> ToBoxed<SyncAction> for T
where
    T: SyncActionNode<Context = SyncActionContext> + 'static,
{
    fn to_boxed(self) -> Box<dyn NodeBase> {
        let node: SyncAction = self.into();
        Box::new(node)
    }
}

// =====================
// Stateful Action Node
// =====================

pub struct StatefulActionContext<T = ()>(pub T);

impl<T> Deref for StatefulActionContext<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for StatefulActionContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct StatefulAction(Box<dyn StatefulActionNode<Context = StatefulActionContext>>);

pub trait StatefulActionNode: std::fmt::Debug + Send + Sync {
    type Context;

    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn on_start(&mut self, ctx: &mut NodeData<Self::Context>) -> NodeResult;

    fn on_running(&mut self, ctx: &mut NodeData<Self::Context>) -> NodeResult;

    #[allow(unused_variables)]
    fn on_halted(&mut self, ctx: &mut NodeData<Self::Context>) -> NodeResult<()> {
        Ok(())
    }
}

impl Deref for StatefulAction {
    type Target = Box<dyn StatefulActionNode<Context = StatefulActionContext>>;

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
        StatefulActionNode::ports(&*self.0)
    }

    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult {
        let prev_status = ctx.status;

        let new_status = match prev_status {
            NodeStatus::Idle => {
                ::log::debug!("[behaviortree_rs]: {}::on_start()", &ctx.config.path);
                // let mut wrapper = ArgWrapper::new(&mut self.data, &mut self.context);
                let new_status =
                    self.on_start(&mut NodeData::new(ctx, &mut StatefulActionContext(())))?;
                // drop(wrapper);
                if matches!(new_status, NodeStatus::Idle) {
                    return Err(NodeError::StatusError(
                        format!("{}::on_start()", ctx.config.path),
                        "Idle".to_string(),
                    ));
                }
                new_status
            }
            NodeStatus::Running => {
                ::log::debug!("[behaviortree_rs]: {}::on_running()", &ctx.config.path);
                let new_status =
                    self.on_running(&mut NodeData::new(ctx, &mut StatefulActionContext(())))?;
                if matches!(new_status, NodeStatus::Idle) {
                    return Err(NodeError::StatusError(
                        format!("{}::on_running()", ctx.config.path),
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
        StatefulActionNode::on_halted(
            &mut *self.0,
            &mut NodeData::new(ctx, &mut StatefulActionContext(())),
        )
    }
}

impl<T> From<T> for StatefulAction
where
    T: StatefulActionNode<Context = StatefulActionContext> + 'static,
{
    fn from(value: T) -> StatefulAction {
        StatefulAction(Box::new(value))
    }
}

impl<T> ToBoxed<StatefulAction> for T
where
    T: StatefulActionNode<Context = StatefulActionContext> + 'static,
{
    fn to_boxed(self) -> Box<dyn NodeBase> {
        let node: StatefulAction = self.into();
        Box::new(node)
    }
}
