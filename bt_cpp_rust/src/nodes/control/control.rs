use std::{cell::RefCell, rc::Rc};

use crate::nodes::{TreeNodeBase, TreeNodePtr, NodeError};

pub trait ControlNodeBase: TreeNodeBase + ControlNode {}

pub type ControlNodePtr = Rc<RefCell<dyn ControlNodeBase>>;

pub trait ControlNode: TreeNodeBase {
    /// Add child to `ControlNode`
    fn add_child(&mut self, child: TreeNodePtr);
    /// Return reference to `Vec` of children nodes
    fn children(&self) -> &Vec<Rc<RefCell<dyn TreeNodeBase>>>;
    /// Control-specific implementation of `halt()`
    fn halt_control(&mut self);
    /// Call `halt()` on child at index
    fn halt_child(&self, index: usize) -> Result<(), NodeError>;
    /// Halt all children at and after index
    fn halt_children(&self, start: usize) -> Result<(), NodeError>;
    /// Reset status of all child nodes
    fn reset_children(&self);
    /// Creates a cloned version of itself as a `ControlNode` trait object
    fn clone_boxed(&self) -> Box<dyn ControlNodeBase>;
}

impl Clone for Box<dyn ControlNodeBase> {
    fn clone(&self) -> Box<dyn ControlNodeBase> {
        self.clone_boxed()
    }
}