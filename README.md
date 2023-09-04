# bt-cpp-rust

Rust implementation of [BehaviorTree.CPP](https://github.com/BehaviorTree/BehaviorTree.CPP). Still a WIP. A table of features can be found below.

## Usage

To create your own custom nodes in `bt-cpp-rust`, you need to derive certain traits which provide automatically-implemented functionality that you won't need to change. These provide access to the blackboard, config, ports, etc. You will also need to implement a few traits based on the type of node you're creating.

### Derive traits

To create your own node, regardless of the type, you need to derive the same 3 traits: 

```rust
#[derive(Clone, Debug, TreeNodeDefaults)]
```

### Implement traits

At the minimum, you also need to implement the `TreeNode` trait. The only required method to implement is `tick()`. If you are using ports with your node, you also need to implement `provided_ports()`. An example of both functions is shown below:

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
```

### Specify Node Type

The other traits to derive are determined by the type of node you are implementing. If you're implementing a `SyncActionNode`, you need to also derive two other traits:

```rust
#[derive(Clone, Debug, TreeNodeDefaults, ActionNode, SyncActionNode)]
```

For other action nodes, replace `SyncActionNode` with the type of action node you're implementing.

# Example

## Rust code
```rust
use std::{cell::RefCell, rc::Rc};

use bt_cpp_rust::{
    basic_types::{NodeStatus, PortsList},
    blackboard::Blackboard,
    macros::{define_ports, input_port, register_node},
    nodes::{NodeConfig, TreeNode, TreeNodeDefaults, NodeError, StatefulActionNode},
    tree::Factory,
};
use bt_derive::{ActionNode, TreeNodeDefaults, StatefulActionNode};
use log::{error, info};

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, StatefulActionNode)]
pub struct DummyActionNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    counter: u32,
    halt_requested: RefCell<bool>,
}

impl DummyActionNode {
    pub fn new(name: &str, config: NodeConfig) -> DummyActionNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
            counter: 0,
            halt_requested: RefCell::new(false),
        }
    }
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

| Feature              | Status |
| -------------------- | ------ |
| SequenceNode         | ✅     |
| ReactiveSequenceNode | ✅     |
| ParallelNode         | ✅     |
| SyncActionNode       | ✅     |
| StatefulActionNode   | ✅     |

