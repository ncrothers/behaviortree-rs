use behaviortree_rs::{
    basic_types::NodeType,
    nodes::{ToBoxed, TreeNode},
    Blackboard, Factory,
};
use behaviortree_rs_derive::register_action_node;
use nodes::StatusNode;

mod nodes;

#[test]
fn visitor() {
    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Sequence>
                    <Sequence>
                        <Inverter>
                            <StatusNode status="Success" />
                        </Inverter>
                        <StatusNode status = "Failure" />
                    </Sequence>
                </Sequence>
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut factory = Factory::new();
    factory.register_node("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let blackboard = Blackboard::create();

    let tree = factory.create_tree_from_text(xml, &blackboard);
    assert!(tree.is_ok());
    let tree = tree.unwrap();

    let nodes: Vec<&str> = tree.visit_nodes().map(|node| node.name()).collect();

    assert_eq!(
        nodes,
        vec![
            "Sequence",
            "Sequence",
            "Inverter",
            "StatusNode",
            "StatusNode"
        ]
    );
}
