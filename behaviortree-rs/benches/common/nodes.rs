use behaviortree_rs::prelude::*;

#[derive(Debug)]
pub struct StatusNode;

impl SyncActionNode for StatusNode {
    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        let status: NodeStatus = ctx.get_input("status")?;

        log::info!("I am a node that returns {}!", status);

        Ok(status)
    }

    fn ports(&self) -> PortsList {
        PortsList::from([PortInfo::input::<NodeStatus>("status").build()])
    }
}
