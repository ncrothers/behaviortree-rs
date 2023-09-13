# bt-cpp-rust

Rust implementation of [BehaviorTree.CPP](https://github.com/BehaviorTree/BehaviorTree.CPP). Still a WIP. A table of features can be found below.

## Usage

To create your own custom nodes in `bt-cpp-rust`, you need to derive certain traits which provide automatically-implemented functionality that you won't need to change. These provide access to the blackboard, config, ports, etc. You will also need to implement a few traits based on the type of node you're creating.

### Creating a node

To create your own node, use the `#[bt_node(...)]` macro. The argument to the macro is the type of node you want to create. The `bt_node` macro modifies your struct, adding fields, method implementations, and trait implementations.

For example, the following node definition:

```rust
use bt_cpp_rust::bt_node;

#[bt_node(SyncActionNode)]
struct DummyActionNode {}
```

Gets expanded to:

```rust
#[derive(Clone, Debug, TreeNodeDefaults, ActionNode, SyncActionNode)]
struct DummyActionNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus
}

impl DummyActionNode {
    pub fn new(name: impl AsRef<str>, config: NodeConfig, status: NodeStatus) -> DummyActionNode {
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

```rust
#[bt_node(SyncActionNode)]
struct DummyActionNode {
    foo: String,
    bar: u32
}
```

Gets expanded to:

```rust
#[derive(Clone, Debug, TreeNodeDefaults, ActionNode, SyncActionNode)]
struct DummyActionNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    foo: String,
    bar: u32
}

impl DummyActionNode {
    pub fn new(name: impl AsRef<str>, config: NodeConfig, status: NodeStatus, foo: String, bar: u32) -> DummyActionNode {
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

```rust
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

```rust
#[bt_node(SyncActionNode)]
struct DummyActionNode {
    #[bt(default = "NodeStatus::Success")]
    foo: NodeStatus,
    #[bt(default)] // defaults to empty String
    bar: String
}
```

### Implement traits

At the minimum, you also need to implement the `TreeNode` and `NodeHalt` traits. The only required method to implement is `tick()`. If you are using ports with your node, you also need to implement `provided_ports()`. You can choose to implement `halt()` if you need to do any cleanup when the node is stopped externally.

Example:

```rust
impl TreeNode for DummyActionNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        // Your code goes here
        // ...

        // You must return a `NodeStatus` or an `Err`.
        Ok(NodeStatus::Success)
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(
            // No default value
            input_port!("foo"),
            // With default value 
            input_port!("bar", 16)
        )
    }
}

impl NodeHalt for DummyActionNode {
    // Only need to add this if you want to clean up.
    // Otherwise just: `impl NodeHalt for DummyActionNode {}` will suffice.
    fn halt(&mut self) {
        // Cleanup code here
        // ...
    }
}
```

# Example

## Rust code
```rust
use std::{cell::RefCell, rc::Rc};

use bt_cpp_rust::{
    basic_types::{NodeStatus, PortsList},
    blackboard::Blackboard,
    macros::{define_ports, input_port, register_node},
    nodes::{TreeNode, TreeNodeDefaults, NodeError, StatefulActionNode},
    tree::Factory,
};
use bt_derive::bt_node;
use log::{error, info};

#[bt_node(StatefulActionNode)]
pub struct DummyActionNode {
    #[bt(default)]
    counter: u32,
    #[bt(default = "RefCell::new(false)")]
    halt_requested: RefCell<bool>,
}

impl StatefulActionNode for DummyActionNode {
    fn on_start(&mut self) -> NodeStatus {
        info!("Starting!");

        NodeStatus::Running
    }

    fn on_running(&mut self) -> NodeStatus {
        info!("Running!");

        NodeStatus::Success
    }
}

impl TreeNode for DummyActionNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let foo = self.config.get_input::<String>("foo");
        info!(
            "{} tick! Counter: {}, blackboard value: {}",
            self.name,
            self.counter,
            foo.unwrap()
        );

        let bar = self.config.get_input::<u32>("bar");
        match bar {
            Ok(bar) => info!("- Blackboard [bar]: {}", bar),
            Err(e) => error!("{e:?}"),
        }

        self.counter += 1;

        self.config.blackboard.borrow_mut().write(
            "bb_test",
            String::from("this value comes from the blackboard!"),
        );

        match self.counter > 2 {
            true => Ok(NodeStatus::Success),
            false => {
                self.config
                    .blackboard
                    .borrow_mut()
                    .write("foo", String::from("new value!"));
                Ok(NodeStatus::Running)
            }
        }
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("foo"), input_port!("bar", 16))
    }
}
```

## XML file

```xml
<root>
    <BehaviorTree ID="main">
        <Sequence>
            <DummyNode foo="hi" bar="128" />
            <DummyNode foo="hi" />
            <CustomNode foo="{bb_test}" />
            <Sequence>
                <InnerNode foo="within inner sequence" />
            </Sequence>
            <SubTree ID="sub1" />
        </Sequence>
    </BehaviorTree>

    <BehaviorTree ID="sub1">
        <Sequence>
            <DummyNode foo="hi" bar="128" />
            <DummyNode foo="last node!" />
            <SubTree ID="sub2" />
        </Sequence>
    </BehaviorTree>

    <BehaviorTree ID="sub2">
        <Sequence>
            <DummyNode foo="hi" bar="128" />
            <DummyNode foo="last node!" />
            <Parallel>
                <DummyNode foo="parallel node!" />
            </Parallel>
        </Sequence>
    </BehaviorTree>
</root>
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
