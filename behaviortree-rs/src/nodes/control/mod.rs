use std::{cell::RefCell, rc::Rc};

use crate::nodes::{NodeError, TreeNodeBase, TreeNodePtr};

mod if_then_else;
use futures::future::BoxFuture;
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

pub trait ControlNodeBase: TreeNodeBase + ControlNode {}

pub type ControlNodePtr = Rc<RefCell<dyn ControlNodeBase>>;

pub trait ControlNode: TreeNodeBase {
    /// Add child to `ControlNode`
    fn add_child(&mut self, child: TreeNodePtr);
    /// Return reference to `Vec` of children nodes
    fn children(&self) -> &Vec<TreeNodePtr>;
    /// Call `halt()` on child at index
    fn halt_child(&mut self, index: usize) -> BoxFuture<Result<(), NodeError>>;
    /// Halt all children at and after index
    fn halt_children(&mut self, start: usize) -> BoxFuture<Result<(), NodeError>>;
    /// Reset status of all child nodes
    fn reset_children(&mut self) -> BoxFuture<()>;
}

// impl Clone for Box<dyn ControlNodeBase + Send + Sync> {
//     fn clone(&self) -> Box<dyn ControlNodeBase + Send + Sync> {
//         self.clone_boxed()
//     }
// }
