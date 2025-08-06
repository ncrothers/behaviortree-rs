use behaviortree_rs::{
    basic_types::NodeType, blackboard::Blackboard, node_registry::NodeRegistry, nodes::ToBoxed,
    tree::Tree,
};

mod nodes;
mod control {
    mod fallback;
}

use nodes::{EchoNode, RunForNode, StatusNode};

#[test]
fn if_then_else() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <IfThenElse>
                    <StatusNode status="Failure" />
                    <EchoNode msg="Success branch" />
                    <EchoNode msg="Failure branch" />
                </IfThenElse>
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::default();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}

#[test]
fn parallel_all() {
    nodes::test_setup();

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}

#[test]
fn parallel() {
    nodes::test_setup();

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}

#[test]
fn reactive_fallback() {
    nodes::test_setup();

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .tree_name("main")
        .build();

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    match tree.tick_while_running() {
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}

#[test]
fn reactive_sequence() {
    nodes::test_setup();

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);
    registry.insert(
        "RunForNode",
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
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}

#[test]
fn sequence_star() {
    nodes::test_setup();

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);
    registry.insert(
        "RunForNode",
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
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}

#[test]
fn sequence_vanilla() {
    nodes::test_setup();

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);
    registry.insert(
        "RunForNode",
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
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}

#[test]
fn while_do_else() {
    nodes::test_setup();

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
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);
    registry.insert(
        "RunForNode",
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
        Ok(status) => log::info!("Final status: {status:?}"),
        Err(e) => log::error!("{e}"),
    }
}
