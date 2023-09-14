use bt_cpp_rust::{macros::register_node, blackboard::Blackboard, tree::Factory, basic_types::NodeStatus};

use crate::nodes::StatusNode;

mod nodes;

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
    "#.to_string();

    let mut factory = Factory::new();
    register_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::new_ptr();

    let tree = factory.create_tree_from_text(xml, &blackboard);

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
    "#.to_string();

    let mut factory = Factory::new();
    register_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::new_ptr();

    let tree = factory.create_tree_from_text(xml, &blackboard);

    assert!(tree.is_err());

    // Check case where there is only one tree, but ID is not specified (Ok)
    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();
    register_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::new_ptr();

    let tree = factory.create_tree_from_text(xml, &blackboard);

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::new_ptr();
    let tree = factory.create_tree_from_text(xml, &blackboard);

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
    "#.to_string();

    let mut factory = Factory::new();

    // Don't register StatusNode

    let blackboard = Blackboard::new_ptr();
    let tree = factory.create_tree_from_text(xml, &blackboard);

    assert!(tree.is_err());
}