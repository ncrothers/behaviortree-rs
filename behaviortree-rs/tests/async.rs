use behaviortree_rs::{
    basic_types::NodeType,
    node_registry::NodeRegistry,
    nodes::{NodeStatus, ToBoxed},
    tree::{Tree, TreeConfig},
    Blackboard,
};
use nodes::StatusNode;

mod nodes;

#[tokio::test]
async fn check_send_sync() {
    let xml = r#"
        <root main_tree_to_execute="main">
            <BehaviorTree ID="main">
                <StatusNode status="Success" />
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();
    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    let blackboard = Blackboard::new();

    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .build();

    let tree = Tree::from_config(&config);

    assert!(tree.is_ok());
    let mut tree = tree.unwrap();

    let res = tokio::spawn(async move { tree.tick_once() }).await.unwrap();

    assert!(matches!(res, Ok(NodeStatus::Success)));
}
