use behaviortree_rs::{
    basic_types::NodeStatus, blackboard::Blackboard, macros::register_action_node, tree::Factory,
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

    let mut factory = Factory::new();
    // register_action_node!(factory, "DataNode", DataNode, String::new());
    register_action_node!(factory, "DataNode", DataNode, String::new());
    register_action_node!(factory, "DataNode2", DataNode, field);
    register_action_node!(factory, "DataNode3", DataNode, field);
    let blackboard = Blackboard::create();

    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();
    register_action_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::create();

    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();
    register_action_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::create();

    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();
    register_action_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::create();

    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();
    register_action_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::create();

    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();
    register_action_node!(factory, "StatusNode", StatusNode);
    let blackboard = Blackboard::create();

    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();

    register_action_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::create();
    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();

    // Don't register StatusNode

    let blackboard = Blackboard::create();
    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();

    register_action_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::create();
    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let mut factory = Factory::new();

    register_action_node!(factory, "StatusNode", StatusNode);
    register_action_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::create();
    let tree = factory.create_sync_tree_from_text(xml, &blackboard);

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

    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(async move {
        let task = tokio::spawn(async move {
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

            let mut factory = Factory::new();

            register_action_node!(factory, "StatusNode", StatusNode);
            register_action_node!(factory, "EchoNode", EchoNode);

            let blackboard = Blackboard::create();
            let tree = factory.create_async_tree_from_text(xml, &blackboard).await;

            if tree.is_err() {
                log::error!("{}", tree.as_ref().err().unwrap());
            }

            assert!(tree.is_ok());

            let mut tree = tree.unwrap();

            let res = tree.tick_once().await;
            assert!(res.is_ok());
        });

        let res = task.await;

        assert!(res.is_ok());
    });
}
