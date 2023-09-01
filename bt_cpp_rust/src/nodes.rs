use std::{collections::HashMap, sync::Arc, rc::Rc, cell::RefCell};

use bt_derive::{ControlNode, TreeNodeDefaults};
use thiserror::Error;

use crate::{blackboard::Blackboard, basic_types::{TreeNodeManifest, NodeStatus, PortsList}, macros::{get_input, define_ports, input_port}};

pub type PortsRemapping = HashMap<String, String>;

pub trait TreeNodeBase: TreeNode + TreeNodeDefaults + NodeClone {}

pub trait TreeNodeDefaults: NodeClone {
    fn status(&self) -> NodeStatus;
    fn reset_status(&mut self);
}

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("Out of bounds index")]
    IndexError,
}

#[derive(Clone, Debug)]
pub enum PreCond {
    FailureIf,
    SuccessIf,
    SkipIf,
    WhileTrue,
    Count,
}

#[derive(Clone, Debug)]
pub enum PostCond {
    OnHalted,
    OnFailure,
    OnSuccess,
    Always,
    Count,
}

#[derive(Clone, Debug)]
pub enum NodeType {
    Control,
    Leaf,
    SubTree,
}

#[derive(Clone, Debug)]
pub struct NodeConfig {
    blackboard: Rc<RefCell<Blackboard>>,
    input_ports: PortsRemapping,
    output_ports: PortsRemapping,
    // manifest: Box<TreeNodeManifest>,
    uid: u16,
    path: String,
    pre_conditions: HashMap<PreCond, String>,
    post_conditions: HashMap<PostCond, String>,
}

impl NodeConfig {
    pub fn new(blackboard: Rc<RefCell<Blackboard>>) -> NodeConfig {
        Self {
            blackboard,
            input_ports: HashMap::new(),
            output_ports: HashMap::new(),
            uid: 1,
            path: String::from("TODO"),
            pre_conditions: HashMap::new(),
            post_conditions: HashMap::new(),
        }
    }
}

pub struct TreeNodeData {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    registration_id: String,
}

impl TreeNodeData {
    pub fn new(name: String, config: NodeConfig) -> TreeNodeData {
        Self {
            name,
            config,
            status: NodeStatus::Idle,
            registration_id: String::new(),
        }
    }
}

pub trait NodeClone {
    fn clone_node(&self) -> Box<dyn TreeNodeBase>;
}

impl<T> NodeClone for T
where
    T: 'static + TreeNodeBase + Clone,
{
    fn clone_node(&self) -> Box<dyn TreeNodeBase> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn TreeNodeBase> {
    fn clone(&self) -> Box<dyn TreeNodeBase> {
        self.clone_node()
    }
}

pub trait TreeNode: std::fmt::Debug + NodeClone {
    fn tick(&mut self) -> NodeStatus;
    fn halt(&mut self) {}
    fn provided_ports(&self) -> PortsList {
        HashMap::new()
    }
}

// pub trait LeafNode: TreeNode + NodeClone {}

pub trait ControlNode {
    /// Add child to `ControlNode`
    fn add_child(&mut self, child: Box<dyn TreeNodeBase>);
    /// Return reference to `Vec` of children nodes
    fn children(&self) -> &Vec<Box<dyn TreeNodeBase>>;
    /// Call `halt()` on child at index
    fn halt_child(&mut self, index: usize) -> Result<(), NodeError>;
    /// Halt all children at and after index
    fn halt_children(&mut self, start: usize) -> Result<(), NodeError>;
    /// Reset status of all child nodes
    fn reset_children(&mut self);
}

#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct SequenceNode {
    config: NodeConfig,
    children: Vec<Box<dyn TreeNodeBase>>,
    status: NodeStatus,
    child_idx: usize,
    all_skipped: bool
}

impl SequenceNode {
    pub fn new(config: NodeConfig) -> SequenceNode {
        Self {
            config,
            children: Vec::new(),
            status: NodeStatus::Idle,
            child_idx: 0,
            all_skipped: false,
        }
    }
}

impl TreeNode for SequenceNode {
    fn tick(&mut self) -> NodeStatus {
        if self.status == NodeStatus::Idle {
            self.all_skipped = true;
        }

        self.status = NodeStatus::Running;

        while self.child_idx < self.children.len() {
            let cur_child = &mut self.children[self.child_idx];

            let _prev_status = cur_child.status();
            let child_status = cur_child.tick();

            self.all_skipped &= child_status == NodeStatus::Skipped;

            match &child_status {
                NodeStatus::Failure => {
                    self.reset_children();
                    self.child_idx = 0;
                    self.child_idx += 1;
                }
                NodeStatus::Success | NodeStatus::Skipped => {
                    self.child_idx += 1;
                }
                _ => {}
            };
        }

        if self.child_idx == self.children.len() {
            self.reset_children();
            self.child_idx = 0;
        }

        NodeStatus::Success
    }

    fn halt(&mut self) {
        self.reset_children()
    }
}

#[derive(Debug, Clone, TreeNodeDefaults)]
pub struct DummyLeafNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    counter: u32,
}

impl DummyLeafNode {
    pub fn new(name: &str, config: NodeConfig) -> DummyLeafNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
            counter: 0,
        }
    }
}

impl TreeNode for DummyLeafNode {
    fn tick(&mut self) -> NodeStatus {
        let foo = get_input!(self, String, "foo");
        println!("{} tick! Counter: {}, blackboard value: {}", self.name, self.counter, foo.unwrap());

        let bar = get_input!(self, u32, "bar");
        println!("- Blackboard [bar]: {}", bar.unwrap());

        self.counter += 1;
        
        match self.counter > 2 {
            true => NodeStatus::Success,
            false => {
                self.config.blackboard.borrow_mut().write("foo", String::from("new value!"));
                NodeStatus::Running
            },
        }
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(
            input_port!("foo"),
            input_port!("bar")
        )
    }
}

