use std::{collections::HashMap, sync::Arc};

use crate::{blackboard::Blackboard, basic_types::{TreeNodeManifest, NodeStatus}};

pub type PortsRemapping = HashMap<String, String>;

pub enum PreCond {
    FailureIf,
    SuccessIf,
    SkipIf,
    WhileTrue,
    Count,
}

pub enum PostCond {
    OnHalted,
    OnFailure,
    OnSuccess,
    Always,
    Count,
}

pub struct NodeConfig {
    blackboard: Blackboard,
    input_ports: PortsRemapping,
    output_ports: PortsRemapping,
    manifest: Box<TreeNodeManifest>,
    uid: u16,
    path: String,
    pre_conditions: HashMap<PreCond, String>,
    post_conditions: HashMap<PostCond, String>,
}

pub type PreTickCallback = fn(&TreeNode) -> NodeStatus;
pub type PostTickCallback = fn(&TreeNode, NodeStatus) -> NodeStatus;

struct TreeNodeData {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    registration_id: String,
    substitution_callback: Option<PreTickCallback>,
    post_condition_callback: Option<PostTickCallback>,
}

impl TreeNodeData {
    pub fn new(name: String, config: NodeConfig) -> TreeNodeData {
        Self {
            name,
            config,
            status: NodeStatus::Idle,
            registration_id: String::new(),
            substitution_callback: None,
            post_condition_callback: None,
        }
    }
}

pub struct TreeNode {
    p: Arc<TreeNodeData>,
}

impl TreeNode {
    pub fn execute_tick(&mut self) -> NodeStatus {
        let mut new_status = self.p.status.clone();
        // check preconditions

        new_status.clone()
    }
}