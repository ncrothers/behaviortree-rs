use std::{any::Any, cell::RefCell, rc::Rc, sync::Arc};

use crate::{nodes::{NodeError, TreeNodeBase, TreeNode}, NodeResult};

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

use super::{HaltFn, NodeConfig, NodeStatus, PortsFn, PortsList, TickFn};

// pub trait ControlNodeBase: TreeNodeBase + ControlNode {}

// pub type ControlNodePtr = Rc<RefCell<dyn ControlNodeBase>>;

// pub trait ControlNode: TreeNodeBase {
//     /// Add child to `ControlNode`
//     fn add_child(&mut self, child: TreeNode);
//     /// Return reference to `Vec` of children nodes
//     fn children(&self) -> &Vec<TreeNode>;
//     /// Call `halt()` on child at index
//     fn halt_child(&mut self, index: usize) -> BoxFuture<Result<(), NodeError>>;
//     /// Halt all children at and after index
//     fn halt_children(&mut self, start: usize) -> BoxFuture<Result<(), NodeError>>;
//     /// Reset status of all child nodes
//     fn reset_children(&mut self) -> BoxFuture<()>;
// }

#[derive(Debug)]
pub struct ControlNode {
    pub name: String,
    pub type_str: String,
    pub config: NodeConfig,
    pub status: NodeStatus,
    /// Vector of child nodes
    pub children: Vec<TreeNode>,
    /// Function pointer to tick
    pub tick_fn: TickFn<ControlNode>,
    /// Function pointer to halt
    pub halt_fn: HaltFn<ControlNode>,
    pub ports_fn: PortsFn,
    pub context: Box<dyn Any + Send>,
}

// fn test<T: Send>(arg: T) {

// }

// fn foo() {
//     let node = ControlNode {
//         name: String::new(),
//         type_str: String::new(),
//         config: ,
//     };
// }

impl ControlNode {
    pub async fn execute_tick(&mut self) -> NodeResult {
        (self.tick_fn)(self).await
    }

    pub async fn halt(&mut self) {
        (self.halt_fn)(self).await
    }

    pub async fn halt_child(&mut self, index: usize) -> NodeResult<()> {
        let child = self.children.get_mut(index).ok_or(NodeError::IndexError)?;
        if child.status() == ::behaviortree_rs::nodes::NodeStatus::Running {
            // let child_ptr: *mut _ = &mut **child;
            child.halt().await;
        }
        Ok(child.reset_status())
    }

    pub async fn halt_children(&mut self, start: usize) -> NodeResult<()> {
        if start >= self.children.len() {
            return Err(NodeError::IndexError);
        }

        let end = self.children.len();

        for i in start..end {
            self.halt_child(i).await?;
        }

        Ok(())
    }

    pub async fn reset_children(&mut self) {
        self.halt_children(0).await.expect("reset_children failed, shouldn't be possible. Report this")
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

