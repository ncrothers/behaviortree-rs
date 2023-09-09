use bt_derive::{DecoratorNode, TreeNodeDefaults};

use crate::{
    basic_types::NodeStatus,
    nodes::{TreeNodeDefaults, DecoratorNode, NodeConfig, TreeNode, TreeNodePtr, NodeError, NodeHalt},
    macros::{define_ports, input_port}
};

/// The RetryNode is used to execute a child several times if it fails.
/// 
/// If the child returns SUCCESS, the loop is stopped and this node
/// returns SUCCESS.
/// 
/// If the child returns FAILURE, this node will try again up to N times
/// (N is read from port "num_attempts").
/// 
/// Example:
/// 
/// ```xml
/// <RetryUntilSuccessful num_attempts="3">
///     <OpenDoor/>
/// </RetryUntilSuccessful>
/// ```
#[derive(TreeNodeDefaults, DecoratorNode, Debug, Clone)]
pub struct RetryNode {
    config: NodeConfig,
    child: Option<TreeNodePtr>,
    status: NodeStatus,
    max_attempts: i32,
    try_count: usize,
    all_skipped: bool,
}

impl RetryNode {
    pub fn new(config: NodeConfig) -> RetryNode {
        Self {
            config,
            child: None,
            status: NodeStatus::Idle,
            max_attempts: -1,
            try_count: 0,
            all_skipped: true,
        }
    }
}

impl TreeNode for RetryNode {
    fn tick(&mut self) -> Result<NodeStatus, NodeError> {
        // Load num_cycles from the port value
        self.max_attempts = self.config.get_input("num_attempts")?;

        let mut do_loop = (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;
        
        if matches!(self.status, NodeStatus::Idle) {
            self.all_skipped = true;
        }

        self.set_status(NodeStatus::Running);

        while do_loop {
            let child_status = self.child.as_ref().unwrap().borrow_mut().execute_tick()?;

            self.all_skipped &= matches!(child_status, NodeStatus::Skipped);

            match child_status {
                NodeStatus::Success => {
                    self.try_count = 0;
                    self.reset_child();
                    
                    return Ok(NodeStatus::Success);
                }
                NodeStatus::Failure => {
                    self.try_count += 1;
                    do_loop = (self.try_count as i32) < self.max_attempts || self.max_attempts == -1;

                    self.reset_child();
                }
                NodeStatus::Running => return Ok(NodeStatus::Running),
                NodeStatus::Skipped => {
                    self.reset_child();

                    return Ok(NodeStatus::Skipped);
                }
                NodeStatus::Idle => return Err(NodeError::StatusError("InverterNode".to_string(), "Idle".to_string())),
            }
        }

        self.try_count = 0;

        match self.all_skipped {
            true => Ok(NodeStatus::Skipped),
            false => Ok(NodeStatus::Failure),
        }
    }

    fn provided_ports(&self) -> crate::basic_types::PortsList {
        define_ports!(
            input_port!("num_attempts")
        )
    }
}

impl NodeHalt for RetryNode {
    fn halt(&mut self) {
        self.try_count = 0;
        self.reset_child();
    }
}