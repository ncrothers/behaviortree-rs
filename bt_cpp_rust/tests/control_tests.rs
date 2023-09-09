use std::rc::Rc;

use bt_cpp_rust::{macros::register_node, tree::Factory, blackboard::Blackboard};
use log::{info, error};

mod nodes;

use nodes::{EchoNode, RunForNode, StatusNode};

#[test]
fn fallback() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <Fallback>
                    <StatusNode status="Failure" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Success" />
                    <StatusNode status="Failure" />
                    <StatusNode status="Success" />
                </Fallback>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("{status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn if_then_else() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let xml = r#"
        <root>
            <BehaviorTree ID="main">
                <IfThenElse>
                    <StatusNode status="Failure" />
                </IfThenElse>
            </BehaviorTree>
        </root>
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn parallel_all() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn parallel() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn reactive_fallback() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn reactive_sequence() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn sequence_star() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn sequence_vanilla() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}

#[test]
fn while_do_else() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

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
    "#.to_string();

    let mut factory = Factory::new();

    register_node!(factory, "StatusNode", StatusNode);
    register_node!(factory, "EchoNode", EchoNode);
    register_node!(factory, "RunForNode", RunForNode);

    let blackboard = Blackboard::new_ptr();

    factory.register_bt_from_text(xml).unwrap();

    let mut tree = factory.instantiate_tree(&blackboard, "main").unwrap();

    match tree.tick_while_running() {
        Ok(status) => info!("Final status: {status:?}"),
        Err(e) => error!("{e}")
    }
}