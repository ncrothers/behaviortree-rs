use std::{collections::HashMap, rc::Rc, cell::RefCell};

use bt_derive::{ControlNode, TreeNodeDefaults, ActionNode};
use log::{info, error};
use thiserror::Error;

use crate::{blackboard::Blackboard, basic_types::{TreeNodeManifest, NodeStatus, PortsList, self, PortDirection, NodeType, PortValue}, macros::{get_input, define_ports, input_port}, tree::{ParseError, TreeNodePtr, ControlNodePtr}};

pub type PortsRemapping = HashMap<String, String>;

pub trait TreeNodeBase: TreeNode + TreeNodeDefaults + GetNodeType {}
pub trait ControlNodeBase: TreeNodeBase + ControlNode {}
pub trait ActionNodeBase: TreeNodeBase + ActionNode {}

pub trait TreeNodeDefaults {
    fn status(&self) -> NodeStatus;
    fn reset_status(&mut self);
    fn config(&mut self) -> &mut NodeConfig;
    fn into_boxed(self) -> Box<dyn TreeNodeBase>;
    fn into_tree_node_ptr(&self) -> TreeNodePtr;
    fn clone_node_boxed(&self) -> Box<dyn TreeNodeBase>;
}
pub trait GetNodeType {
    fn node_type(&self) -> basic_types::NodeType;
}

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("Out of bounds index")]
    IndexError,
    #[error("Couldn't find port [{0}]")]
    PortError(String),
    #[error("Couldn't parse port [{0}] value into specified type [{1}]")]
    PortValueParseError(String, String),
    #[error("Couldn't find entry in blackboard [{0}]")]
    BlackboardError(String),
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
pub struct NodeConfig {
    blackboard: Rc<RefCell<Blackboard>>,
    input_ports: PortsRemapping,
    output_ports: PortsRemapping,
    manifest: Option<Rc<TreeNodeManifest>>,
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
            manifest: None,
            uid: 1,
            path: String::from("TODO"),
            pre_conditions: HashMap::new(),
            post_conditions: HashMap::new(),
        }
    }

    pub fn add_port(&mut self, direction: PortDirection, name: String, value: String) {
        match direction {
            PortDirection::Input => {
                self.input_ports.insert(name, value);
            }
            PortDirection::Output => {
                self.output_ports.insert(name, value);
            }
            _ => {}
        };
    }

    pub fn has_port(&self, direction: &PortDirection, name: &String) -> bool {
        match direction {
            PortDirection::Input => {
                self.input_ports.contains_key(name)
            }
            PortDirection::Output => {
                self.output_ports.contains_key(name)

            }
            _ => false
        }
    }

    pub fn manifest(&self) -> Result<Rc<TreeNodeManifest>, ParseError> {
        match self.manifest.as_ref() {
            Some(manifest) => Ok(Rc::clone(manifest)),
            None => Err(ParseError::InternalError(format!("Missing manifest. This shouldn't happen; please report this.")))
        }
    }

    pub fn set_manifest(&mut self, manifest: Rc<TreeNodeManifest>) {
        let _ = self.manifest.insert(manifest);
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

pub trait PortClone {
    fn clone_port(&self) -> Box<dyn PortValue>;
}

// impl<T> PortClone for T
// where
//     T: 'static + PortValue + Clone,
// {
//     fn clone_port(&self) -> Box<dyn PortValue> {
//         Box::new(self.clone())
//     }
// }

impl Clone for Box<dyn PortValue> {
    fn clone(&self) -> Box<dyn PortValue> {
        self.clone_port()
    }
}

impl Clone for Box<dyn TreeNodeBase> {
    fn clone(&self) -> Box<dyn TreeNodeBase> {
        self.clone_node_boxed()
    }
}

impl Clone for Box<dyn ActionNodeBase> {
    fn clone(&self) -> Box<dyn ActionNodeBase> {
        self.clone_boxed()
    }
}

impl Clone for Box<dyn ControlNodeBase> {
    fn clone(&self) -> Box<dyn ControlNodeBase> {
        self.clone_boxed()
    }
}

pub trait TreeNode: std::fmt::Debug {
    fn tick(&mut self) -> NodeStatus;
    fn halt(&mut self) {}
    fn provided_ports(&self) -> PortsList {
        HashMap::new()
    }
}

// pub trait LeafNode: TreeNode + NodeClone {}

pub trait ControlNode: TreeNodeBase {
    /// Add child to `ControlNode`
    fn add_child(&mut self, child: TreeNodePtr);
    /// Return reference to `Vec` of children nodes
    fn children(&self) -> &Vec<TreeNodePtr>;
    /// Call `halt()` on child at index
    fn halt_child(&mut self, index: usize) -> Result<(), NodeError>;
    /// Halt all children at and after index
    fn halt_children(&mut self, start: usize) -> Result<(), NodeError>;
    /// Reset status of all child nodes
    fn reset_children(&mut self);
    fn clone_boxed(&self) -> Box<dyn ControlNodeBase>;
}

pub trait ActionNode {
    fn clone_boxed(&self) -> Box<dyn ActionNodeBase>;
}

pub trait ConditionNode {}

pub trait DecoratorNode {}

pub trait SubTreeNode {}

#[derive(TreeNodeDefaults, ControlNode, Debug, Clone)]
pub struct SequenceNode {
    config: NodeConfig,
    children: Vec<TreeNodePtr>,
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

            let _prev_status = cur_child.borrow().status();
            let child_status = cur_child.borrow_mut().tick();

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

#[derive(Debug, Clone, TreeNodeDefaults, ActionNode)]
pub struct DummyActionNode {
    name: String,
    config: NodeConfig,
    status: NodeStatus,
    counter: u32,
}

impl DummyActionNode {
    pub fn new(name: &str, config: NodeConfig) -> DummyActionNode {
        Self {
            name: name.to_string(),
            config,
            status: NodeStatus::Idle,
            counter: 0,
        }
    }
}

impl TreeNode for DummyActionNode {
    fn tick(&mut self) -> NodeStatus {
        let foo = get_input!(self, String, "foo");
        info!("{} tick! Counter: {}, blackboard value: {}", self.name, self.counter, foo.unwrap());

        let bar = get_input!(self, u32, "bar");
        match bar {
            Ok(bar) => info!("- Blackboard [bar]: {}", bar),
            Err(e) => error!("{e:?}")
        }

        self.counter += 1;

        self.config.blackboard.borrow_mut().write("bb_test", String::from("this value comes from the blackboard!"));
        
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
            input_port!("bar", 16)
        )
    }
}

