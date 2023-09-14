use bt_cpp_rust::{macros::register_node, tree::Factory, blackboard::Blackboard, basic_types::NodeStatus};
use log::{info, error};

mod nodes;

use nodes::{RunForNode, StatusNode};

use crate::nodes::SuccessThenFailure;

#[test]
fn force_failure() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <ForceFailure>
                    <StatusNode status="Success" />
                </ForceFailure>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            info!("{status:?}");

            assert!(matches!(status, NodeStatus::Failure));
        }
        Err(e) => error!("{e}")
    }
}

#[test]
fn force_success() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <ForceSuccess>
                    <StatusNode status="Failure" />
                </ForceSuccess>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            info!("{status:?}");

            assert!(matches!(status, NodeStatus::Success));
        }
        Err(e) => error!("{e}")
    }
}

#[test]
fn inverter() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Inverter>
                    <StatusNode status="Success" />
                </Inverter>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            info!("{status:?}");

            assert!(matches!(status, NodeStatus::Failure));
        }
        Err(e) => error!("{e}")
    }
}

#[test]
fn keep_running_until_failure() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <KeepRunningUntilFailure>
                    <Sequence>
                        <RunFor iters="2" status="Success" />
                        <RunFor iters="2" status="Failure" />
                    </Sequence>
                </KeepRunningUntilFailure>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "RunFor", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            info!("{status:?}");
        }
        Err(e) => error!("{e}")
    }
}

#[test]
fn repeat() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Repeat num_cycles="5">
                    <SuccessThenFailure iters="3" />
                </Repeat>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "RunFor", RunForNode);
    register_node!(factory, "SuccessThenFailure", SuccessThenFailure);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            info!("{status:?}");

            assert!(matches!(status, NodeStatus::Failure));
        }
        Err(e) => error!("{e}")
    }
}

#[test]
fn retry() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Retry num_attempts="5">
                    <Inverter>
                        <SuccessThenFailure iters="3" />
                    </Inverter>
                </Retry>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "RunFor", RunForNode);
    register_node!(factory, "SuccessThenFailure", SuccessThenFailure);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            info!("{status:?}");

            assert!(matches!(status, NodeStatus::Success));
        }
        Err(e) => error!("{e}")
    }
}

#[test]
fn run_once() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <RunOnce then_skip="true">
                    <SuccessThenFailure iters="3" />
                </RunOnce>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "RunFor", RunForNode);
    register_node!(factory, "SuccessThenFailure", SuccessThenFailure);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            info!("{status:?}");

            assert!(matches!(status, NodeStatus::Success));
        }
        Err(e) => error!("{e}")
    }
}