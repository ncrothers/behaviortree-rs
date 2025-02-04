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

use std::ops::{Deref, DerefMut};

use super::{
    NodeBase, NodeData, NodeDataGeneric, NodeError, NodeResult, NodeStatus, PortsList, ToBoxed,
};

pub struct ControlContext;

impl NodeData<'_, ControlContext> {
    /// Halt children from this index to the end.
    ///
    /// # Errors
    ///
    /// Returns `NodeError::IndexError` if `start` is out of bounds.
    pub fn halt_children(&mut self, start: usize) -> NodeResult<()> {
        if start >= self.children.len() {
            return Err(NodeError::IndexError);
        }

        let end = self.children.len();

        for i in start..end {
            self.halt_child(i)?;
        }

        Ok(())
    }

    /// Halts and resets all children
    pub fn reset_children(&mut self) -> NodeResult<()> {
        self.halt_children(0)
    }

    /// Halt child at the `index`. Not to be confused with `halt_child()`, which is
    /// a helper that calls `halt_child_idx(0)`, primarily used for `Decorator` nodes.
    pub fn halt_child(&mut self, index: usize) -> NodeResult<()> {
        let child = self.children.get_mut(index).ok_or(NodeError::IndexError)?;
        if child.status() == ::behaviortree_rs::nodes::NodeStatus::Running {
            child.halt()?;
        }
        child.reset_status();
        Ok(())
    }
}

#[derive(Debug)]
pub struct Control(Box<dyn ControlNode>);

pub trait ControlNode: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn tick(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult;

    #[allow(unused_variables)]
    fn halt(&mut self, ctx: &mut NodeData<ControlContext>) -> NodeResult<()> {
        Ok(())
    }
}

impl Deref for Control {
    type Target = Box<dyn ControlNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Control {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NodeBase for Control {
    fn ports(&self) -> PortsList {
        ControlNode::ports(&*self.0)
    }

    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult {
        self.tick(&mut NodeData::new(ctx))
    }

    fn halt(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult<()> {
        let mut ctx = NodeData::new(ctx);

        ControlNode::halt(&mut *self.0, &mut ctx)?;
        ctx.set_status(NodeStatus::Idle);
        ctx.reset_children()?;

        Ok(())
    }
}

impl<T> From<T> for Control
where
    T: ControlNode + 'static,
{
    fn from(value: T) -> Control {
        Control(Box::new(value))
    }
}

impl<T> ToBoxed<Control> for T
where
    T: ControlNode + 'static,
{
    fn to_boxed(self) -> Box<dyn NodeBase> {
        let node: Control = self.into();
        Box::new(node)
    }
}
