extern crate self as bt_cpp_rust;

pub mod basic_types;
pub mod blackboard;

pub mod nodes;

pub mod macros;
pub mod tree;

pub mod derive {
    pub use bt_derive::*;
}

// Re-exports for convenience
pub use tree::Factory;
pub use blackboard::Blackboard;
pub use derive::bt_node;

extern crate futures as futures_internal;
extern crate tokio as tokio_internal;

pub mod sync {
    pub use futures::{future::BoxFuture, executor::block_on};
    
    pub use tokio::sync::Mutex;
    pub use tokio::task::spawn_blocking;
}
