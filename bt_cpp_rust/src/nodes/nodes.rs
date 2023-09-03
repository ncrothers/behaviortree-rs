use std::{collections::HashMap, rc::Rc, cell::RefCell};

use log::{info, error};
use thiserror::Error;

use crate::{blackboard::Blackboard, basic_types::{TreeNodeManifest, NodeStatus, PortsList, self, PortDirection, PortValue, PortsRemapping}, tree::ParseError};


// =============================
// Trait Definitions
// =============================

pub trait TreeNodeBase: TreeNode + TreeNodeDefaults + GetNodeType {}
pub trait ControlNodeBase: TreeNodeBase + ControlNode {}
pub trait ActionNodeBase: TreeNodeBase + ActionNode {}

pub type TreeNodePtr = Rc<RefCell<dyn TreeNodeBase>>;
pub type ControlNodePtr = Rc<RefCell<dyn ControlNodeBase>>;

pub trait TreeNode: std::fmt::Debug {
    fn tick(&mut self) -> NodeStatus;
    fn halt(&mut self) {}
    fn provided_ports(&self) -> PortsList {
        HashMap::new()
    }
}

pub trait TreeNodeDefaults {
    fn status(&self) -> NodeStatus;
    fn reset_status(&mut self);
    fn config(&mut self) -> &mut NodeConfig;
    fn into_boxed(self) -> Box<dyn TreeNodeBase>;
    fn into_tree_node_ptr(&self) -> TreeNodePtr;
    fn clone_node_boxed(&self) -> Box<dyn TreeNodeBase>;
}

pub trait ControlNode: TreeNodeBase {
    /// Add child to `ControlNode`
    fn add_child(&mut self, child: TreeNodePtr);
    /// Return reference to `Vec` of children nodes
    fn children(&self) -> &Vec<TreeNodePtr>;
    /// Call `halt()` on child at index
    fn halt_child(&self, index: usize) -> Result<(), NodeError>;
    /// Halt all children at and after index
    fn halt_children(&self, start: usize) -> Result<(), NodeError>;
    /// Reset status of all child nodes
    fn reset_children(&self);
    /// Creates a cloned version of itself as a `ControlNode` trait object
    fn clone_boxed(&self) -> Box<dyn ControlNodeBase>;
}

pub trait ActionNode {
    /// Creates a cloned version of itself as a `ActionNode` trait object
    fn clone_boxed(&self) -> Box<dyn ActionNodeBase>;
}

pub trait ConditionNode {}

pub trait DecoratorNode {}

pub trait SubTreeNode {}

pub trait GetNodeType {
    fn node_type(&self) -> basic_types::NodeType;
}

// =============================
// Enum Definitions
// =============================

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

// =========================================
// Struct Definitions and Implementations
// =========================================

#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub blackboard: Rc<RefCell<Blackboard>>,
    pub input_ports: PortsRemapping,
    pub output_ports: PortsRemapping,
    pub manifest: Option<Rc<TreeNodeManifest>>,
    pub uid: u16,
    pub path: String,
    pub pre_conditions: HashMap<PreCond, String>,
    pub post_conditions: HashMap<PostCond, String>,
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

    pub fn blackboard(&self) -> &Rc<RefCell<Blackboard>> {
        &self.blackboard
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
