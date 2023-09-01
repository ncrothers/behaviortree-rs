use std::{collections::HashMap, sync::Arc, rc::Rc, cell::RefCell};

use thiserror::Error;

use crate::{blackboard::Blackboard, basic_types::{TreeNodeManifest, NodeStatus, PortsList}, define_ports, input_port};

pub type PortsRemapping = HashMap<String, String>;

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
    fn clone_node(&self) -> Box<dyn TreeNode>;
}

impl<T> NodeClone for T
where
    T: 'static + TreeNode + Clone,
{
    fn clone_node(&self) -> Box<dyn TreeNode> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn TreeNode> {
    fn clone(&self) -> Box<dyn TreeNode> {
        self.clone_node()
    }
}

pub trait TreeNode: std::fmt::Debug + NodeClone {
    fn tick(&mut self) -> NodeStatus;
    fn halt(&mut self) {}
    fn status(&self) -> NodeStatus;
    fn reset_status(&mut self);
    fn provided_ports(&self) -> PortsList;
}

// pub trait LeafNode: TreeNode + NodeClone {}

pub trait ControlNode: TreeNode {
    /// Add child to `ControlNode`
    fn add_child(&mut self, child: Box<dyn TreeNode>);
    /// Return reference to `Vec` of children nodes
    fn children(&self) -> &Vec<Box<dyn TreeNode>>;
    /// Call `halt()` on child at index
    fn halt_child(&mut self, child_idx: usize) -> Result<(), NodeError>;
    /// Halt all children at and after index
    fn halt_children(&mut self, start: usize) -> Result<(), NodeError>;
    /// Reset status of all child nodes
    fn reset_children(&mut self);
    fn as_treenode_ref(&self) -> &dyn TreeNode;
}

#[derive(Debug, Clone)]
pub struct SequenceNode {
    config: NodeConfig,
    children: Vec<Box<dyn TreeNode>>,
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

    fn status(&self) -> NodeStatus {
        self.status.clone()
    }

    fn reset_status(&mut self) {
        self.status = NodeStatus::Idle;
    }

    fn halt(&mut self) {
        self.reset_children()
    }

    fn provided_ports(&self) -> PortsList {
        todo!()
    }
}

impl ControlNode for SequenceNode {
    fn add_child(&mut self, child: Box<dyn TreeNode>) {
        self.children.push(child);
    }

    fn children(&self) -> &Vec<Box<dyn TreeNode>> {
        &self.children
    }

    fn halt_child(&mut self, index: usize) -> Result<(), NodeError> {
        match self.children.get_mut(index) {
            Some(child) => Ok(child.halt()),
            None => Err(NodeError::IndexError),
        }
    }

    fn halt_children(&mut self, start: usize) -> Result<(), NodeError> {
        if start >= self.children.len() {
            return Err(NodeError::IndexError);
        }

        self.children[start..].iter_mut().for_each(|child| child.halt());
        Ok(())
    }

    fn reset_children(&mut self) {
        self
            .children
            .iter_mut()
            .for_each(|child| child.reset_status());
    }

    fn as_treenode_ref(&self) -> &dyn TreeNode {
        self
    }
}

#[derive(Debug)]
pub enum Node {
    SubTree(String),
    ControlNode(Box<dyn ControlNode>),
    LeafNode(Box<dyn TreeNode>),
    Empty,
}

#[derive(Debug, Clone)]
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
        println!("{} tick! Counter: {}, blackboard value: {}", self.name, self.counter, self.config.blackboard.borrow().read::<String>("foo").unwrap());

        self.counter += 1;
        
        match self.counter > 2 {
            true => NodeStatus::Success,
            false => {
                self.config.blackboard.borrow_mut().write("foo", String::from("new value!"));
                NodeStatus::Running
            },
        }
    }

    fn status(&self) -> NodeStatus {
        self.status.clone()
    }

    fn reset_status(&mut self) {
        self.status = NodeStatus::Idle;
    }

    fn provided_ports(&self) -> PortsList {
        define_ports!(
            input_port!("foo")
        )
    }
}