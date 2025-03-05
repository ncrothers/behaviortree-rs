use behaviortree_rs::prelude::*;
use behaviortree_rs_derive::{BTToString, FromString};

#[derive(BTToString)]
struct Test {}

impl ToString for Test {
    fn to_string(&self) -> String {
        todo!()
    }
}

pub fn test_setup() {
    let _ = pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();
}

#[derive(Debug)]
pub struct StatusNode;

impl SyncActionNode for StatusNode {
    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        let status: NodeStatus = ctx.get_input("status")?;

        log::info!("I am a node that returns {}!", status.bt_to_string());

        Ok(status)
    }

    fn ports(&self) -> PortsList {
        PortsList::from([PortInfo::input::<NodeStatus>("status").build()])
    }
}

#[derive(Debug, Default)]
pub struct SuccessThenFailure {
    iter: usize,
}

impl SyncActionNode for SuccessThenFailure {
    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        let max_iters: usize = ctx.get_input("iters")?;

        log::info!("SuccessThenFailure!");

        if self.iter < max_iters {
            self.iter += 1;
            Ok(NodeStatus::Success)
        } else {
            Ok(NodeStatus::Failure)
        }
    }

    fn ports(&self) -> PortsList {
        PortsList::from([PortInfo::input::<usize>("iters").build()])
    }
}

#[derive(Debug)]
pub struct EchoNode;

impl SyncActionNode for EchoNode {
    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        let msg: String = ctx.get_input("msg")?;

        log::info!("{msg}");

        Ok(NodeStatus::Success)
    }

    fn ports(&self) -> PortsList {
        PortsList::from([PortInfo::input::<String>("msg").build()])
    }
}

#[derive(Debug, Default)]
pub struct RunForNode {
    counter: usize,
}

impl StatefulActionNode for RunForNode {
    fn ports(&self) -> PortsList {
        PortsList::from([
            PortInfo::input::<usize>("iters").build(),
            PortInfo::input::<NodeStatus>("status")
                .default_value(NodeStatus::Success)
                .build(),
        ])
    }

    fn on_start(&mut self, ctx: &mut NodeData<StatefulActionContext>) -> NodeResult {
        log::info!("on_start()");

        Ok(NodeStatus::Running)
    }

    fn on_running(&mut self, ctx: &mut NodeData<StatefulActionContext>) -> NodeResult {
        let limit: usize = ctx.get_input("iters")?;

        if self.counter < limit {
            log::info!("RunFor {}", self.counter);
            self.counter += 1;
            Ok(NodeStatus::Running)
        } else {
            Ok(ctx.get_input("status")?)
        }
    }
}

#[derive(Debug)]
pub struct DataNode {
    inner_name: String,
}

impl DataNode {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            inner_name: value.into(),
        }
    }
}

impl SyncActionNode for DataNode {
    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        Ok(NodeStatus::Success)
    }
}

#[derive(Debug, Default)]
pub struct StringAppendNode {}

impl SyncActionNode for StringAppendNode {
    fn ports(&self) -> PortsList {
        PortsList::from([
            PortInfo::input::<String>("key").build(),
            PortInfo::input::<String>("char").build(),
        ])
    }

    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        let key = ctx.get_input::<String>("key")?;
        let char = ctx.get_input::<String>("char")?;

        let value = ctx.blackboard.get::<String>(&key).unwrap_or_default();

        let value = value + &char;

        ctx.blackboard.set(key, value);

        Ok(NodeStatus::Success)
    }
}
