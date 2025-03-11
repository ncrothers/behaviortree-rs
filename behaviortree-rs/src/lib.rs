//! # behaviortree-rs
//!
//! Rust implementation of [BehaviorTree.CPP](https://github.com/BehaviorTree/BehaviorTree.CPP). Still a WIP. A table of features can be found below.
//!
//! ## Usage
//!
//! TODO

pub mod basic_types;
pub mod blackboard;
pub mod value;

pub mod nodes;

pub mod error;
pub mod node_registry;
pub(crate) mod parser;
pub mod tree;

pub mod scripting;

pub mod derive {
    pub use behaviortree_rs_derive::*;
}

// Re-exports for convenience
pub use blackboard::Blackboard;
pub use nodes::NodeResult;

pub mod prelude {
    pub use crate::basic_types::{NodeStatus, NodeType, PortInfo, PortsList};
    pub use crate::blackboard::Blackboard;
    pub use crate::node_registry::NodeRegistry;
    pub use crate::nodes::*;
    pub use crate::tree::{Tree, TreeConfig};
}
