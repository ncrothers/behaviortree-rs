mod force_failure;
use std::ops::{Deref, DerefMut};

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

use super::{NodeBase, NodeData, NodeResult, NodeType, PortsList, ToBoxed};

#[derive(Debug)]
pub struct Decorator(Box<dyn DecoratorNode>);

pub trait DecoratorNode: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult;

    #[allow(unused_variables)]
    fn halt(&mut self, ctx: &mut NodeData) -> NodeResult<()> {
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
    fn node_type(&self) -> NodeType {
        NodeType::Decorator
    }

    fn ports(&self) -> PortsList {
        DecoratorNode::ports(&*self.0)
    }

    fn execute_tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        self.tick(ctx)
    }

    fn halt(&mut self, ctx: &mut NodeData) -> NodeResult<()> {
        DecoratorNode::halt(&mut *self.0, ctx)
    }
}

impl<T> From<T> for Decorator
where
    T: DecoratorNode + 'static,
{
    fn from(value: T) -> Decorator {
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
