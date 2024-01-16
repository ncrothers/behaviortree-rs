/*!
# behaviortree-rs

Rust implementation of [BehaviorTree.CPP](https://github.com/BehaviorTree/BehaviorTree.CPP). Still a WIP. A table of features can be found below.

## Usage

To create your own custom nodes in `behaviortree-rs`, you need to derive certain traits which provide automatically-implemented functionality that you won't need to change. These provide access to the blackboard, config, ports, etc. You will also need to implement a few traits based on the type of node you're creating.

### Creating a node

To create your own node, use the `#[bt_node(...)]` macro. The argument to the macro is the type of node you want to create. The `bt_node` macro modifies your struct, adding fields, method implementations, and trait implementations.

For example, the following node definition:

```ignore
use behaviortree_rs::bt_node;

#[bt_node(SyncActionNode)]
struct DummyActionNode {}
```

Gets expanded to:

```ignore
#[derive(Clone, Debug, TreeNodeDefaults, ActionNode, SyncActionNode)]
struct DummyActionNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus
}

impl DummyActionNode {
    pub fn new(name: impl AsRef<str>, config: NodeConfig) -> DummyActionNode {
        Self {
            name: name.as_ref().to_string(),
            config,
            status: NodeStatus::Idle
        }
    }
}
```

You are allowed to create this definition yourself, but it is _highly recommended_ that you use `#[bt_node(...)]` for simplicity and ease of node creation.

Of course, you can add your own fields to the struct, which get included in the generated struct. Just add them to the definition, and the generated code will reflect it:

```ignore
#[bt_node(SyncActionNode)]
struct DummyActionNode {
    foo: String,
    bar: u32
}
```

Gets expanded to:

```ignore
#[derive(Clone, Debug, TreeNodeDefaults, ActionNode, SyncActionNode)]
struct DummyActionNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    foo: String,
    bar: u32
}

impl DummyActionNode {
    pub fn new(name: impl AsRef<str>, config: NodeConfig, foo: String, bar: u32) -> DummyActionNode {
        Self {
            name: name.as_ref().to_string(),
            config,
            status: NodeStatus::Idle,
            foo,
            bar
        }
    }
}
```

As you can see, by default any fields you add to the struct will be added to the parameters of `new()`. If you don't want the ability to set a field manually at initialization time, add the `#[bt(default)]` attribute. Just writing `#[bt(default)]` will call `<type>::default()`, which only works if the specified type implements the `Default` trait. To specify an explicit default value: `#[bt(default = "10")]`. Notice the value is wrapped in quotes, so the text in the quotes will be evaluated as Rust code. The valid options to provide as a default are:

```ignore
// Function calls
#[bt(default = "String::from(10)")]

// Variables
#[bt(default = "foo")]

// Paths (like enums)
#[bt(default = "NodeStatus::Idle")]

// Literals
#[bt(default = "10")]
```

An example in practice:

```ignore
use behaviortree_rs::bt_node;

#[bt_node(SyncActionNode)]
struct DummyActionNode {
    #[bt(default = "NodeStatus::Success")]
    foo: NodeStatus,
    #[bt(default)] // defaults to empty String
    bar: String
}
```

### Async vs Sync

At this moment, all nodes are implemented as `async` behind the scenes. However, when building your own nodes you have the choice to implement it as either sync or async. By default, `behaviortree_rs` will expect you to implement the `async` version of the required traits. However, you can specify this explicitly by adding keywords to the `#[bt_node(...)]` macro.

```ignore
# use behaviortree_rs::bt_node;
// Default behavior
#[bt_node(SyncActionNode, Async)]
struct DummyActionNode {}

// Require implementation of the sync version of the traits
#[bt_node(SyncActionNode, Sync)]
struct DummyActionNode {}
```

You'll see how the implementation differs between the two in the next section.

### Implement traits

You have the choice of implementing either a synchronous or asynchronous `tick()` and `halt()` method. If you are doing any I/O operations (network calls, file operations, etc.), especially those that use an `async` interface, you should implement the `async` version (which is the default unless you specify otherwise). For very simple nodes, you can just implement the sync version to avoid the minor extra boilerplate for the `async` methods.

Based on the runtime style you choose, you need to implement two traits:
- Async: `AsyncTick` and `AsyncHalt`
- Sync: `SyncTick` and `SyncHalt`

You also need to implement the `NodePorts` trait regardless of sync vs. async. The details for each of these traits is detailed below.

### `AsyncTick`

```rust
use behaviortree_rs::{
    bt_node,
    nodes::{AsyncTick, AsyncHalt, NodeStatus, NodeError, PortsList, NodePorts},
    macros::{define_ports, input_port, output_port},
    sync::BoxFuture,
};

#[bt_node(SyncActionNode)]
struct DummyActionStruct {}

impl AsyncTick for DummyActionStruct {
    fn tick(&mut self) -> BoxFuture<Result<NodeStatus, NodeError>> {
        Box::pin(async move {
            // Some implementation
            // ...

            // You must return a `NodeStatus` (i.e. Failure, Success, Running, or Skipped)
            // Or an Err
            Ok(NodeStatus::Success)
        })
    }
}

// If you don't use any ports, this can be left empty
// impl NodePorts for DummyActionStruct {}
impl NodePorts for DummyActionStruct {
    fn provided_ports(&self) -> PortsList {
        define_ports!(
            // No default value
            input_port!("foo"),
            // With default value 
            input_port!("bar", 16)
        )
    }
}

// If you don't need to do cleanup, leave as-is
impl AsyncHalt for DummyActionStruct {}
```

### `SyncTick`

TODO: Currently doesn't compile. Need to address

```ignore
use behaviortree_rs::{
    bt_node,
    nodes::{SyncTick, SyncHalt, NodeStatus, NodeError, PortsList, NodePorts},
    macros::{define_ports, input_port, output_port},
};

#[bt_node(SyncActionNode, Sync)]
struct DummyActionStruct {}

impl SyncTick for DummyActionStruct {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        // Some implementation
        // ...

        // You must return a `NodeStatus` (i.e. Failure, Success, Running, or Skipped)
        // Or an Err
        Ok(NodeStatus::Success)
    }
}

// If you don't use any ports, this can be left empty
// impl NodePorts for DummyActionStruct {}
impl NodePorts for DummyActionStruct {
    fn provided_ports(&self) -> PortsList {
        define_ports!(
            // No default value
            input_port!("foo"),
            // With default value 
            input_port!("bar", 16)
        )
    }
}

// If you don't need to do cleanup, leave as-is
impl SyncHalt for DummyActionStruct {}
```
*/

extern crate self as behaviortree_rs;

pub mod basic_types;
pub mod blackboard;

pub mod nodes;

pub mod macros;
pub mod tree;

pub mod derive {
    pub use behaviortree_rs_derive::*;
}

// Re-exports for convenience
pub use blackboard::Blackboard;
pub use derive::bt_node;
pub use tree::Factory;
pub use nodes::NodeResult;

extern crate futures as futures_internal;
extern crate tokio as tokio_internal;

pub mod sync {
    pub use futures::{executor::block_on, future::BoxFuture};

    pub use tokio::sync::Mutex;
    pub use tokio::task::spawn_blocking;
}
