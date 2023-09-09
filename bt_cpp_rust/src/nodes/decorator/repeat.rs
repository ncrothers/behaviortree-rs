use bt_derive::{DecoratorNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
    macros::{define_ports, input_port}
};

/// /// The RetryNode is used to execute a child several times, as long
/// as it succeed.
/// 
/// To succeed, the child must return SUCCESS N times (port "num_cycles").
/// 
/// If the child returns FAILURE, the loop is stopped and this node
/// returns FAILURE.
/// 
/// Example:
/// 
/// ```xml
/// <Repeat num_cycles="3">
///   <ClapYourHandsOnce/>
/// </Repeat>
/// ```
#[derive(TreeNodeDefaults, DecoratorNode, Debug, Clone)]
pub struct RepeatNode {
    config: NodeConfig,
    child: Option<TreeNodePtr>,
    status: NodeStatus,
    num_cycles: i32,
    repeat_count: usize,
    all_skipped: bool,
}

impl RepeatNode {
    pub fn new(config: NodeConfig) -> RepeatNode {
        Self {
            config,
            child: None,
            status: NodeStatus::Idle,
            num_cycles: -1,
            repeat_count: 0,
            all_skipped: true,
        }
    }
}

impl TreeNode for RepeatNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        // Load num_cycles from the port value
        self.num_cycles = self.config.get_input("num_cycles")?;

        let mut do_loop = (self.repeat_count as i32) < self.num_cycles || self.num_cycles == -1;
        
        if matches!(self.status, NodeStatus::Idle) {
            self.all_skipped = true;
        }

        self.set_status(NodeStatus::Running);

        while do_loop {
            let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick()?;

            self.all_skipped &= matches!(child_status, NodeStatus::Skipped);

            match child_status {
                NodeStatus::Success => {
                    self.repeat_count += 1;
                    do_loop = (self.repeat_count as i32) < self.num_cycles || self.num_cycles == -1;

                    self.reset_child();
                }
                NodeStatus::Failure => {
                    self.repeat_count = 0;
                    self.reset_child();

                    return Ok(NodeStatus::Failure);
                }
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Skipped => {
                    self.reset_child();

                    return Ok(NodeStatus::Skipped);
                }
                NodeStatus::Idle => return Err(NodeError::StatusError("InverterNode".to_string(), "Idle".to_string())),
            }
        }

        self.repeat_count = 0;

        match self.all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Success),
        }
    }

    fn provided_ports(&self) -> crate::basic_types::PortsList {
        define_ports!(
            input_port!("num_cycles")
        )
    }
}

impl NodeHalt for RepeatNode {
    fn halt(&mut self) {
        self.repeat_count = 0;
        self.reset_child();
    }
}