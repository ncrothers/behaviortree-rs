use behaviortree_rs::{basic_types::BTToString, prelude::*};

#[derive(Debug)]
pub struct StatusNode;

impl SyncActionNode for StatusNode {
    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        let status: NodeStatus = ctx.get_input("status")?;

        log::info!("I am a node that returns {}!", status.bt_to_string());

        Ok(status)
    }

    fn ports(&self) -> PortsList {
        define_ports!(input_port!("status"))
    }
}