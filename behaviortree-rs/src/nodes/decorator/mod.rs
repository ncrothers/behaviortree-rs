use std::{cell::RefCell, rc::Rc};

use crate::nodes::{NodeError, TreeNodeBase, TreeNodePtr};

mod force_failure;
pub use force_failure::*;
mod force_success;
pub use force_success::*;
mod inverter;
use futures::future::BoxFuture;
pub use inverter::*;
mod keep_running_until_failure;
pub use keep_running_until_failure::*;
mod repeat;
pub use repeat::*;
mod retry;
pub use retry::*;
mod run_once;
pub use run_once::*;

pub trait DecoratorNodeBase: TreeNodeBase + DecoratorNode {}

pub type DecoratorNodePtr = Rc<RefCell<dyn DecoratorNodeBase>>;

pub trait DecoratorNode: TreeNodeBase {
    /// Set child node for `Decorator`
    fn set_child(&mut self, child: TreeNodePtr);
    /// Return reference to child
    fn child(&self) -> Result<&TreeNodePtr, NodeError>;
    /// Call `halt()` on child, same as `reset_child()`
    fn halt_child(&mut self) -> BoxFuture<()>;
    /// Reset status of child and call `halt()`
    fn reset_child(&mut self) -> BoxFuture<()>;
}
