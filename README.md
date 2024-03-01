# behaviortree-rs

Rust implementation of [BehaviorTree.CPP](https://github.com/BehaviorTree/BehaviorTree.CPP). Still a WIP. A table of features can be found below.

## Usage

To create your own custom nodes in `behaviortree-rs`, you need to use the provided attribute macro to transform your `struct` and `impl` blocks. You also need to implement certain functions for each node type, plus the option to implement some optional functions.

### Creating a node

To create your own node, use the `#[bt_node(...)]` macro. The argument to the macro is the type of node you want to create. The `bt_node` macro modifies your struct, adding fields, and method implementations.

For example, the following node definition:

```ignore
use behaviortree_rs::bt_node;

#[bt_node(SyncActionNode)]
struct DummyActionNode {}

#[bt_node(SyncActionNode)]
impl DummyActionNode {
    // Implementations go here
}
```

Of course, you can add your own fields to the struct, which get included in the generated struct. When you add fields, you have the option to require their definition in the node constructor, or have a default value that is populated without the ability to modify when instantiating the node.

```ignore
#[bt_node(SyncActionNode)]
struct DummyActionNode {
    foo: String,
    bar: u32
}
```

If you don't want the ability to set a field manually at initialization time, add the `#[bt(default)]` attribute. Just writing `#[bt(default)]` will call `<type>::default()`, which only works if the specified type implements the `Default` trait. To specify an explicit default value: `#[bt(default = "10")]`. Notice the value is wrapped in quotes, so the text in the quotes will be evaluated as Rust code. The valid options to provide as a default are:

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

### Node functions

```rust
use behaviortree_rs::prelude::*;

#[bt_node(SyncActionNode)]
struct DummyActionStruct {}

#[bt_node(
    SyncActionNode,
    ports = ports,
    tick = tick,
)]
impl DummyActionStruct {
    async fn tick(&mut self) -> NodeResult {
        // Some implementation
        // ...

        // You must return a `NodeStatus` (i.e. Failure, Success, Running, or Skipped)
        // Or an Err
        Ok(NodeStatus::Success)
    }

    fn ports() -> PortsList {
        define_ports!(
            // No default value
            input_port!("foo"),
            // With default value
            input_port!("bar", 16)
        )
    }
}
```

# Feature Progress

✅: Supported
🔴: Not supported

## General features

| Feature              | Status |
| -------------------- | ------ |
| XML parsing          | ✅     |
| Ports                | ✅     |
| Port remapping       | ✅     |
| SubTrees             | ✅     |
| Blackboard           | ✅     |
| &nbsp;               |        |
| XML generation       | 🔴    |
| Scripting            | 🔴    |
| Pre-/post-conditions | 🔴    |
| Loggers/Observers    | 🔴    |
| Substitution rules   | 🔴    |

## Built-in node implementations

| Feature                 | Status |
| ----------------------- | ------ |
| __Control__             |        |
| Fallback                | ✅     |
| ReactiveFallback        | ✅     |
| IfThenElse              | ✅     |
| Sequence                | ✅     |
| ReactiveSequence        | ✅     |
| SequenceStar            | ✅     |
| WhileDoElse             | ✅     |
| Parallel                | ✅     |
| ParallelAll             | ✅     |
|                         |        |
| __Decorator__           |        |
| ForceFailure            | ✅     |
| ForceSuccess            | ✅     |
| Inverter                | ✅     |
| KeepRunningUntilFailure | ✅     |
| Repeat                  | ✅     |
| Retry                   | ✅     |
| RunOnce                 | ✅     |
|                         |        |
| __Action Traits__       |        |
| SyncActionNode          | ✅     |
| StatefulActionNode      | ✅     |
