use behaviortree_rs::{basic_types::NodeType, nodes::ToBoxed, prelude::*};

use crate::nodes::{self, StatusNode, StringAppendNode};

#[test]
fn fallback() {
    nodes::test_setup();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Fallback>
                    <StatusNode status="Failure" />
                    <StringAppend key="str" char="a" />
                    <StatusNode status="Failure" />
                    <StringAppend key="str" char="b" />
                    
                    <StatusNode status="Success" />
                    <StringAppend key="str" char="c" />
                    <StatusNode status="Failure" />
                    <StringAppend key="str" char="d" />
                    <StatusNode status="Success" />
                    <StringAppend key="str" char="e" />
                </Fallback>
            </BehaviorTree>
        </root>
    "#
    .to_string();

    let mut registry = NodeRegistry::default();

    registry.insert("StatusNode", || StatusNode.to_boxed(), NodeType::Action);
    registry.insert(
        "StringAppend",
        || StringAppendNode::default().to_boxed(),
        NodeType::Action,
    );

    let blackboard = Blackboard::create();

    let config = TreeConfig::builder()
        .blackboard(blackboard)
        .registry(&registry)
        .xml(&xml)
        .tree_name("main")
        .build();

    let mut tree = Tree::from_config(&config).unwrap();

    match tree.tick_while_running() {
        Ok(status) => log::info!("{status:?}"),
        Err(e) => log::error!("{e}"),
    }
}
