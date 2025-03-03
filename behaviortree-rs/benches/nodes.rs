use behaviortree_rs::{basic_types::BTToString, prelude::*};

#[bt_node(SyncActionNode)]
pub struct StatusNode {

}

#[bt_node(SyncActionNode)]
impl StatusNode {
    async fn tick(&mut self) -> NodeResult {
        let status: NodeStatus = node_.config.get_input("status")?;

        log::info!("I am a node that returns {}!", status.bt_to_string());

        Ok(status)
    }

    fn ports() -> PortsList {
        define_ports!(input_port!("status"))
    }
}
