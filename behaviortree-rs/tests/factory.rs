use behaviortree_rs::{
    basic_types::{NodeStatus, NodeType},
    blackboard::Blackboard,
    node_registry::NodeRegistry,
    nodes::ToBoxed,
    tree::{Tree, TreeConfig},
};

use crate::nodes::{DataNode, EchoNode, StatusNode};

mod nodes;

#[test]
fn registering() {
    nodes::test_setup();

    // Check case where there is more than one tree, and the ID is specified (Ok)
    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <SubTree ID="secondary" />
            </BehaviorTree>

            <BehaviorTree ID="secondary">
                <DataNode />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let field = "hello".to_string();

    let mut registry = NodeRegistry::default();
    registry.insert(
        "DataNode",
        || DataNode::new("").to_boxed(),
        NodeType::Action,
    );
    let field_clone = field.clone();
    registry.insert(
        "DataNode2",
        move || DataNode::new(field_clone.clone()).to_boxed(),
        NodeType::Action,
    );
    let field_clone = field.clone();
    registry.insert(
        "DataNode3",
        move || DataNode::new(field_clone.clone()).to_boxed(),
        NodeType::Action,
    );

    let config = TreeConfig::builder().registry(&registry).xml(&xml).build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_ok());

    // Check case where there is more than one tree, but ID is not specified (Err)
    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <SubTree ID="secondary" />
            </BehaviorTree>

            <BehaviorTree ID="secondary">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();
    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let blackboard = Blackboard::create();

    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_err());

    // Check case where there is only one tree, but ID is not specified (Ok)
    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();
    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let blackboard = Blackboard::create();

    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_ok());
}

#[test]
fn main_tree_attr() {
    nodes::test_setup();

    // Check case where there is more than one tree, and the ID is specified (Ok)
    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <SubTree ID="secondary" />
            </BehaviorTree>

            <BehaviorTree ID="secondary">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();
    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let blackboard = Blackboard::create();

    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_ok());

    // Check case where there is more than one tree, but ID is not specified (Err)
    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <SubTree ID="secondary" />
            </BehaviorTree>

            <BehaviorTree ID="secondary">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();
    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let blackboard = Blackboard::create();

    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_err());

    // Check case where there is only one tree, but ID is not specified (Ok)
    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();
    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let blackboard = Blackboard::create();

    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_ok());
}

#[test]
fn subtrees() {
    nodes::test_setup();

    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <SubTree ID="one" />
            </BehaviorTree>

            <BehaviorTree ID="one">
                <SubTree ID="two" />
            </BehaviorTree>

            <BehaviorTree ID="two">
                <StatusNode status="Failure" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::create();
    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    let status = tree.tick_while_running();

    assert!(status.is_ok());
    let status = status.unwrap();

    assert!(matches!(status, NodeStatus::Failure));
}

#[test]
fn node_not_registered() {
    nodes::test_setup();

    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <StatusNode status="Failure" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let registry = NodeRegistry::default();

    // Don't register StatusNode

    let blackboard = Blackboard::create();
    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_err());
}

#[test]
fn ignore_treenodesmodel() {
    nodes::test_setup();

    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <StatusNode status="Failure" />
            </BehaviorTree>

            <TreeNodesModel>
                <Action></Action>
            </TreeNodesModel>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::create();
    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    if tree.is_err() {
        log::error!("{}", tree.as_ref().err().unwrap());
    }

    assert!(tree.is_ok());
}

#[test]
fn load_adjacent_controls() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(false)
        .try_init();

    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <Sequence>
                    <Fallback>
                        <Fallback>
                            <StatusNode status="Failure" />
                        </Fallback>
                    </Fallback>
                    <Fallback>
                        <EchoNode msg="hello"/>
                    </Fallback>
                </Sequence>
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::create();
    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    if tree.is_err() {
        log::error!("{}", tree.as_ref().err().unwrap());
    }

    assert!(tree.is_ok());
}

#[test]
fn async_test() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(false)
        .try_init();

    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <Sequence>
                    <Fallback>
                        <Fallback>
                            <StatusNode status="Failure" />
                        </Fallback>
                    </Fallback>
                    <Fallback>
                        <EchoNode msg="hello"/>
                    </Fallback>
                </Sequence>
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert("EchoNode", || EchoNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::create();
    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    if tree.is_err() {
        log::error!("{}", tree.as_ref().err().unwrap());
    }

    assert!(tree.is_ok());

    let mut tree = tree.unwrap();

    let res = tree.tick_once();
    assert!(res.is_ok());
}

#[test]
fn condition() {
    nodes::test_setup();

    // Check case where there is more than one tree, and the ID is specified (Ok)
    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <Fallback>
                    <Condition expr="{count:int} < 5" />
                    <StatusNode status="Failure" />
                </Fallback>
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();
    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let mut blackboard = Blackboard::create();

    let config = TreeConfig::builder()
        .blackboard(blackboard.clone())
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_ok());

    let mut tree = tree.unwrap();

    blackboard.set("count", 6i64);

    let res = tree.tick_once();
    if res.is_err() {
        log::error!("{res:?}");
    }
    assert!(res.is_ok());
    let res = res.unwrap();

    assert_eq!(res, NodeStatus::Failure);

    blackboard.set("count", 2i64);

    let res = tree.tick_once();
    assert!(res.is_ok());
    let res = res.unwrap();

    assert_eq!(res, NodeStatus::Success);
}
