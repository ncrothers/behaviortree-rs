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

use super::{NodeBase, NodeData, NodeResult, NodeType, PortsList, ToBoxed};

#[derive(Debug)]
pub struct Control(Box<dyn ControlNode>);

pub trait ControlNode: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList {
        PortsList::default()
    }

    fn tick(&mut self, ctx: &mut NodeData) -> NodeResult;

    #[allow(unused_variables)]
    fn halt(&mut self, ctx: &mut NodeData) -> NodeResult<()> {
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
    fn node_type(&self) -> NodeType {
        NodeType::Control
    }

    fn ports(&self) -> PortsList {
        ControlNode::ports(&*self.0)
    }

    fn execute_tick(&mut self, ctx: &mut NodeData) -> NodeResult {
        self.tick(ctx)
    }

    fn halt(&mut self, ctx: &mut NodeData) -> NodeResult<()> {
        ControlNode::halt(&mut *self.0, ctx)
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
