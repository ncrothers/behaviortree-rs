use std::{collections::HashMap, rc::Rc, cell::RefCell};

use bt_derive::{ControlNode, TreeNodeDefaults, ActionNode};
use log::info;
use thiserror::Error;

use crate::{blackboard::Blackboard, basic_types::{TreeNodeManifest, NodeStatus, PortsList, self, PortDirection, NodeType, PortValue}, macros::{get_input, define_ports, input_port}, tree::{ParseError, TreeNodePtr}};

pub type PortsRemapping = HashMap<String, String>;

pub trait TreeNodeBase: TreeNode + TreeNodeDefaults + NodeClone + GetNodeType {}

pub trait TreeNodeDefaults: NodeClone {
    fn status(&self) -> NodeStatus;
    fn reset_status(&mut self);
    fn config(&mut self) -> &mut NodeConfig;
}
pub trait GetNodeType {
    fn node_type(&self) -> basic_types::NodeType;
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

impl<T> PortClone for T
where
    T: 'static + PortValue + Clone,
{
    fn clone_port(&self) -> Box<dyn PortValue> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn PortValue> {
    fn clone(&self) -> Box<dyn PortValue> {
        self.clone_port()
    }
}

pub trait NodeClone {
    fn clone_node(&self) -> TreeNodePtr;
}

impl<T> NodeClone for T
where
    T: 'static + TreeNodeBase + Clone,
{
    fn clone_node(&self) -> TreeNodePtr {
        Rc::new(RefCell::new(self.clone()))
    }
}

// pub trait RcDeepClone {
//     fn rc_deep_clone(&self) -> Self;
// }

// impl RcDeepClone for TreeNodePtr {
//     fn rc_deep_clone(&self) -> Self {
//         Rc::new(self.borrow().clone_node())
//     }
// }

// impl Clone for TreeNodePtr {
//     fn clone(&self) -> TreeNodePtr {
//         self.clone_node()
//     }
// }

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
    fn add_child(&mut self, child: TreeNodePtr);
    /// Return reference to `Vec` of children nodes
    fn children(&self) -> &Vec<TreeNodePtr>;
    /// Call `halt()` on child at index
    fn halt_child(&mut self, index: usize) -> Result<(), NodeError>;
    /// Halt all children at and after index
    fn halt_children(&mut self, start: usize) -> Result<(), NodeError>;
    /// Reset status of all child nodes
    fn reset_children(&mut self);
}

pub trait ActionNode {}

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
        info!("{} tick! Counter: {}, blackboard value: {}", self.name, self.counter, foo.unwrap());

        let bar = get_input!(self, u32, "bar");
        info!("- Blackboard [bar]: {}", bar.unwrap());

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

