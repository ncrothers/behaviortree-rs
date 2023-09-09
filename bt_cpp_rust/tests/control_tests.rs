use std::{cell::RefCell, rc::Rc};

use bt_cpp_rust::{nodes::{NodeConfig, NodeError, TreeNode, NodeHalt, StatefulActionNode}, basic_types::{NodeStatus, PortsList, BTToString}, macros::{define_ports, input_port, register_node}, tree::Factory, blackboard::Blackboard};
use bt_derive::{SyncActionNode, ActionNode, TreeNodeDefaults, StatefulActionNode};
use log::{info, error};


#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, SyncActionNode)]
pub struct StatusNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
}

impl StatusNode {
    pub fn new(name: &str, config: NodeConfig) -> StatusNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
        }
    }
}

impl TreeNode for StatusNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let status: NodeStatus = self.config.get_input("status")?;

        info!("I am a node that returns {}!", status.bt_to_string());

        Ok(status)
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("status"))
    }
}

impl NodeHalt for StatusNode {}

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, SyncActionNode)]
pub struct EchoNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
}

impl EchoNode {
    pub fn new(name: &str, config: NodeConfig) -> EchoNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
        }
    }
}

impl TreeNode for EchoNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        let msg: String = self.config.get_input("msg")?;

        info!("{msg}");

        Ok(NodeStatus::Success)
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("msg"))
    }
}

impl NodeHalt for EchoNode {}

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode, StatefulActionNode)]
pub struct RunForNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    counter: usize,
    halt_requested: RefCell<bool>,
}

impl RunForNode {
    pub fn new(name: &str, config: NodeConfig) -> RunForNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
            counter: 0,
            halt_requested: RefCell::new(false),
        }
    }
}

impl TreeNode for RunForNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        Ok(NodeStatus::Idle)
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(input_port!("iters"))
    }
}

impl StatefulActionNode for RunForNode {
    fn on_start(&mut self) -> Result<NodeStatus, NodeError> {
        info!("on_start()");

        Ok(NodeStatus::Running)
    }

    fn on_running(&mut self) -> Result<NodeStatus, NodeError> {
        let limit: usize = self.config.get_input("iters")?;

        if self.counter < limit {
            info!("RunFor {}", self.counter);
            self.counter += 1;
            Ok(NodeStatus::Running)
        }
        else {
            Ok(NodeStatus::Success)
        }
    }
}

#[test]
fn fallback() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Fallback>
                    <StatusNode status="Failure" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Success" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Success" />
                </Fallback>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("{status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn if_then_else() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <IfThenElse>
                    <StatusNode status="Failure" />
                </IfThenElse>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn parallel_all() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <ParallelAll max_failures="-2">
                    <StatusNode status="Success" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Failure" />
                </ParallelAll>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn parallel() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Parallel failure_count="2" success_count="-1">
                    <StatusNode status="Success" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Success" />
                    <StatusNode status="Success" />
                </Parallel>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn reactive_fallback() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <ReactiveFallback>
                    <StatusNode status="Failure" />
                    <EchoNode msg="I am echoing!" />
                    <EchoNode msg="I should not echo!" />
                </ReactiveFallback>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn reactive_sequence() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <ReactiveSequence>
                    <StatusNode status="Success" />
                    <EchoNode msg="I should echo every time!" />
                    <RunForNode iters="3" />
                    <EchoNode msg="I should only echo after 3 iters!" />
                </ReactiveSequence>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn sequence_star() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <SequenceStar>
                    <StatusNode status="Success" />
                    <EchoNode msg="I should echo only once!" />
                    <RunForNode iters="3" />
                    <EchoNode msg="I should be the last echo!" />
                </SequenceStar>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn sequence_vanilla() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Sequence>
                    <StatusNode status="Success" />
                    <EchoNode msg="I should echo only once!" />
                    <RunForNode iters="3" />
                    <EchoNode msg="I should be the last echo!" />
                </Sequence>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn while_do_else() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <WhileDoElse>
                    <StatusNode status="Failure" />
                    <RunForNode iters="3" />
                    <EchoNode msg="I should echo when StatusNode == Failure!" />
                </WhileDoElse>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}