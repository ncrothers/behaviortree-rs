use behaviortree_rs::{
    basic_types::{NodeStatus, NodeType},
    blackboard::Blackboard,
    node_registry::NodeRegistry,
    nodes::ToBoxed,
    tree::Tree,
};

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            log::info!("{status:?}");

            assert!(matches!(status, NodeStatus::Failure));
        }
        Err(e) => log::error!("{e}"),
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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            log::info!("{status:?}");

            assert!(matches!(status, NodeStatus::Success));
        }
        Err(e) => log::error!("{e}"),
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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            log::info!("{status:?}");

            assert!(matches!(status, NodeStatus::Failure));
        }
        Err(e) => log::error!("{e}"),
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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert(
        "RunFor",
        || RunForNode::default().to_boxed(),
        NodeType::Action,
    );

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            log::info!("{status:?}");
        }
        Err(e) => log::error!("{e}"),
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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert(
        "RunForNode",
        || RunForNode::default().to_boxed(),
        NodeType::Action,
    );
    registry.insert(
        "SuccessThenFailure",
        || SuccessThenFailure::default().to_boxed(),
        NodeType::Action,
    );

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            log::info!("{status:?}");

            assert!(matches!(status, NodeStatus::Failure));
        }
        Err(e) => log::error!("{e}"),
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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert(
        "RunForNode",
        || RunForNode::default().to_boxed(),
        NodeType::Action,
    );
    registry.insert(
        "SuccessThenFailure",
        || SuccessThenFailure::default().to_boxed(),
        NodeType::Action,
    );

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            log::info!("{status:?}");

            assert!(matches!(status, NodeStatus::Success));
        }
        Err(e) => log::error!("{e}"),
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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert(
        "RunForNode",
        || RunForNode::default().to_boxed(),
        NodeType::Action,
    );
    registry.insert(
        "SuccessThenFailure",
        || SuccessThenFailure::default().to_boxed(),
        NodeType::Action,
    );

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => {
            log::info!("{status:?}");

            assert!(matches!(status, NodeStatus::Success));
        }
        Err(e) => log::error!("{e}"),
    }
}
