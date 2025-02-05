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
mod subtree;
pub(crate) use subtree::*;

use std::ops::{Deref, DerefMut};

use super::{
    NodeBase, NodeData, NodeDataGeneric, NodeResult, NodeStatus, PortsList, ToBoxed, TreeNode,
};

#[derive(Debug)]
pub struct DecoratorContext;

impl NodeData<'_, DecoratorContext> {
    /// Calls `halt_child_idx(0)`. This should only be used in
    /// `Decorator` nodes
    pub fn halt_child(&mut self) -> NodeResult<()> {
        self.reset_child()
    }

    /// Halts and resets the first child. This should only be used in
    /// `Decorator` nodes
    pub fn reset_child(&mut self) -> NodeResult<()> {
        if let Some(child) = self.children.get_mut(0) {
            if matches!(child.status(), NodeStatus::Running) {
                child.halt()?;
            }

            child.reset_status();
        }

        Ok(())
    }

    /// Gets a mutable reference to the Decorator's child
    ///
    /// # Panics
    ///
    /// This function will panic if the node has no child, but this should
    /// never happen. Decorator child constraints are validated during parsing.
    pub fn child_mut(&mut self) -> &mut TreeNode {
        self.children
            .get_mut(0)
            .expect("Decorator node must have a child, this shouldn't happen")
    }

    /// Gets an immutable reference to the Decorator's child
    ///
    /// # Panics
    ///
    /// This function will panic if the node has no child, but this should
    /// never happen. Decorator child constraints are validated during parsing.
    pub fn child(&self) -> &TreeNode {
        self.children
            .first()
            .expect("Decorator node must have a child, this shouldn't happen")
    }
}

#[derive(Debug)]
pub struct Decorator(Box<dyn DecoratorNode>);

pub trait DecoratorNode: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn tick(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult;

    #[allow(unused_variables)]
    fn halt(&mut self, ctx: &mut NodeData<DecoratorContext>) -> NodeResult<()> {
        Ok(())
    }
}

impl Deref for Decorator {
    type Target = Box<dyn DecoratorNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Decorator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl NodeBase for Decorator {
    fn ports(&self) -> PortsList {
        DecoratorNode::ports(&*self.0)
    }

    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult {
        self.tick(&mut NodeData::new(ctx))
    }

    fn halt(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult<()> {
        let mut ctx = NodeData::new(ctx);

        DecoratorNode::halt(&mut *self.0, &mut ctx)?;

        ctx.reset_child()?;
        ctx.set_status(NodeStatus::Idle);

        Ok(())
    }
}

impl<T> From<T> for Decorator
where
    T: DecoratorNode + 'static,
{
    fn from(value: T) -> Self {
        Decorator(Box::new(value))
    }
}

impl<T> ToBoxed<Decorator> for T
where
    T: DecoratorNode + 'static,
{
    fn to_boxed(self) -> Box<dyn NodeBase> {
        let node: Decorator = self.into();
        Box::new(node)
    }
}
