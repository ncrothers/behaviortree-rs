use behaviortree_rs::{
    basic_types::NodeType,
    node_registry::NodeRegistry,
    nodes::ToBoxed,
    tree::{Tree, TreeConfig},
    Blackboard,
};
use nodes::StatusNode;

mod nodes;

#[test]
fn visitor() {
    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="subtree">
                <StatusNode status="Success" />
            </BehaviorTree>

            <BehaviorTree ID="main">
                <Sequence>
                    <Sequence>
                        <Inverter>
                            <StatusNode status="Success" />
                        </Inverter>
                        <StatusNode status = "Failure" />
                        <SubTree ID="subtree" />
                    </Sequence>
                </Sequence>
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
    let tree = tree.unwrap();

    let nodes: Vec<&str> = tree.visit_nodes().map(|node| node.name()).collect();

    assert_eq!(
        nodes,
        vec![
            "Sequence",
            "Sequence",
            "Inverter",
            "StatusNode",
            "StatusNode",
            "subtree",
            "StatusNode",
        ]
    );
}
