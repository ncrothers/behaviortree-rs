use std::{cell::RefCell, rc::Rc};

use bt_cpp_rust::{
    basic_types::{NodeStatus, PortsList},
    blackboard::Blackboard,
    macros::{define_ports, input_port, register_node},
    nodes::{ActionNode, NodeConfig, TreeNode, TreeNodeDefaults},
    tree::Factory,
};
use bt_derive::{ActionNode, TreeNodeDefaults};
use log::{error, info};

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode)]
pub struct DummyActionNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    counter: u32,
}

impl DummyActionNode {
    pub fn new(name: &str, config: NodeConfig) -> DummyActionNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
            counter: 0,
        }
    }
}

impl TreeNode for DummyActionNode {
    fn tick(&mut self) -> NodeStatus {
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
            true => NodeStatus::Success,
            false => {
                self.config
                    .blackboard
                    .borrow_mut()
                    .write("foo", String::from("new value!"));
                NodeStatus::Running
            }
        }
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("foo"), input_port!("bar", 16))
    }
}

#[test]
fn tree_test() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let text = std::fs::read_to_string("./test.xml").unwrap();
    let mut factory = Factory::new();

    register_node!(factory, "DummyNode", DummyActionNode);
    register_node!(factory, "CustomNode", DummyActionNode);
    register_node!(factory, "InnerNode", DummyActionNode);

    let blackboard = Rc::new(RefCell::new(Blackboard::new()));

    factory.register_bt_from_text(text).unwrap();

    let mut tree = match factory.instantiate_tree(&blackboard, "main") {
        Ok(tree) => tree,
        Err(e) => {
            error!("Error: {e}");
            panic!("");
        }
    };
    info!("{tree:?}");

    let status = tree.tick_while_running();
    info!("{status:?}");
}

#[test]
fn blackboard_test() {
    let status = NodeStatus::Success;
    println!("{}", status.into_string_color());
    let status = NodeStatus::Failure;
    println!("{}", status.into_string_color());
    let status = NodeStatus::Idle;
    println!("{}", status.into_string_color());

    // let dir = PortDirection::Input;
    // let port_info = PortInfo::new(PortDirection::Input, "hello");

    let mut blackboard = Blackboard::new();

    blackboard.write("test", "foo");
    blackboard.write("test int", 500u64);
    blackboard.write("test custom", status);
    blackboard.write("test string_into", "1;2;3;4");

    let val = blackboard.read::<String>("test");
    println!("{:?}", val.unwrap());
    let val = blackboard.read::<u64>("test int");
    println!("{:?}", val.unwrap());
    let val = blackboard.read::<NodeStatus>("test custom");
    println!("{:?}", val.unwrap());
    let val = blackboard.read::<Vec<String>>("test string_into");
    println!("{:?}", val.unwrap());
}
