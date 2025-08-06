use behaviortree_rs::{
    basic_types::{NodeStatus, NodeType},
    blackboard::Blackboard,
    node_registry::NodeRegistry,
    nodes::ToBoxed,
    tree::Tree,
};
use rstest::rstest;

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

    let tree = Tree::builder(&xml, &registry).build();

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
    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

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
    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

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
    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

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
    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

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
    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

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

    let blackboard = Blackboard::new();
    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

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

    let blackboard = Blackboard::new();
    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

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

    let blackboard = Blackboard::new();
    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

    if tree.is_err() {
        log::error!("{}", tree.as_ref().err().unwrap());
    }

    assert!(tree.is_ok());
}

#[test]
fn load_adjacent_controls() {
    nodes::test_setup();

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

    let blackboard = Blackboard::new();
    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

    if tree.is_err() {
        log::error!("{}", tree.as_ref().err().unwrap());
    }

    assert!(tree.is_ok());
}

#[test]
fn async_test() {
    nodes::test_setup();

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

    let blackboard = Blackboard::new();
    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

    if tree.is_err() {
        log::error!("{}", tree.as_ref().err().unwrap());
    }

    assert!(tree.is_ok());

    let mut tree = tree.unwrap();

    let res = tree.tick_once();
    assert!(res.is_ok());
}

#[cfg(feature = "expr")]
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
    let blackboard = Blackboard::new();

    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard.clone())
        .build();

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

#[rstest]
#[case::missing_root(
    r#"
        <root>
        </root>
    "#,
    false
)]
#[case::missing_behavior_tree(
    r#"
        <BehaviorTree ID="main">
            <StatusNode status="Success" />
        </BehaviorTree>
    "#,
    false
)]
#[case::multiple_root_tags(
    r#"
        <root main_tree_to_execute="invalid">
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
        <root main_tree_to_execute="invalid">
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#,
    false
)]
#[case::incorrect_behaviortree_name(
    r#"
        <root main_tree_to_execute="invalid">
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#,
    false
)]
#[case::empty_behavior_tree(
    r#"
        <root>
            <BehaviorTree ID="main">
            </BehaviorTree>
        </root>
    "#,
    false
)]
#[case::empty_subtree(
    r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="two">
            </BehaviorTree>

            <BehaviorTree ID="main">
                <SubTree ID="two" />
            </BehaviorTree>
        </root>
    "#,
    false
)]
#[case::single_node(
    r#"
        <root>
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#,
    true
)]
#[case::simple_subtree(
    r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="two">
                <StatusNode status="Success" />
            </BehaviorTree>

            <BehaviorTree ID="main">
                <SubTree ID="two" />
            </BehaviorTree>
        </root>
    "#,
    true
)]
#[case::decorator_no_child(
    r#"
        <root>
            <BehaviorTree ID="main">
                <Inverter>
                </Inverter>
            </BehaviorTree>
        </root>
    "#,
    false
)]
#[case::decorator_single_child(
    r#"
        <root>
            <BehaviorTree ID="main">
                <Inverter>
                    <StatusNode status="Success" />
                </Inverter>
            </BehaviorTree>
        </root>
    "#,
    true
)]
#[case::decorator_multiple_children(
    r#"
        <root>
            <BehaviorTree ID="main">
                <Inverter>
                    <StatusNode status="Success" />
                    <StatusNode status="Success" />
                </Inverter>
            </BehaviorTree>
        </root>
    "#,
    false
)]
#[case::control_no_child(
    r#"
        <root>
            <BehaviorTree ID="main">
                <Sequence>
                </Sequence>
            </BehaviorTree>
        </root>
    "#,
    false
)]
#[case::control_single_child(
    r#"
        <root>
            <BehaviorTree ID="main">
                <Sequence>
                    <StatusNode status="Success" />
                </Sequence>
            </BehaviorTree>
        </root>
    "#,
    true
)]
fn parsing(#[case] xml: &str, #[case] is_ok: bool) {
    nodes::test_setup();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);

    let blackboard = Blackboard::new();
    let tree = Tree::builder(&xml, &registry)
        .blackboard(blackboard)
        .build();

    assert_eq!(tree.is_ok(), is_ok);
}
