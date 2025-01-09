use std::{
    any::TypeId,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use thiserror::Error;

use crate::{
    basic_types::{
        get_remapped_key, FromString, NodeType, ParseStr, PortDirection, PortValue, PortsRemapping,
        TreeNodeManifest,
    },
    blackboard::BlackboardString,
    tree::ParseError,
    Blackboard,
};

pub use crate::basic_types::{NodeStatus, PortsList};

pub mod action;
pub mod control;
pub mod decorator;

pub type NodeResult<Output = NodeStatus> = Result<Output, NodeError>;

pub trait NodeBase: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList;
    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult;
    fn halt(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult<()>;
}

pub trait ToBoxed<T> {
    fn to_boxed(self) -> Box<dyn NodeBase>;
}

#[derive(Debug)]
pub struct NodeDataGeneric {
    /// Name of the node as registered in the `Factory`
    pub name: String,
    /// The type of this node
    pub node_type: NodeType,
    ///
    pub config: NodeConfig,
    pub status: NodeStatus,
    /// Vector of child nodes
    pub children: Vec<TreeNode>,
}

pub struct NodeData<'a, T> {
    data: &'a mut NodeDataGeneric,
    pub context: &'a mut T,
}

impl<'a, T> Deref for NodeData<'a, T> {
    type Target = NodeDataGeneric;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T> DerefMut for NodeData<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, T> NodeData<'a, T> {
    pub fn new(data: &'a mut NodeDataGeneric, context: &'a mut T) -> Self {
        Self { data, context }
    }
}

#[derive(Debug)]
pub struct TreeNode {
    pub data: NodeDataGeneric,
    pub node: Box<dyn NodeBase>,
}

impl TreeNode {
    /// Returns the current node's status
    pub fn status(&self) -> NodeStatus {
        self.data.status()
    }

    /// Resets the status back to `NodeStatus::Idle`
    pub fn reset_status(&mut self) {
        self.data.reset_status();
    }

    /// Update the node's status
    pub fn set_status(&mut self, status: NodeStatus) {
        self.data.set_status(status);
    }

    /// Tick the node
    pub fn execute_tick(&mut self) -> NodeResult {
        self.node.execute_tick(&mut self.data)
    }

    /// Halt the node
    pub fn halt(&mut self) -> NodeResult<()> {
        self.node.halt(&mut self.data)
    }

    /// Get the name of the node
    pub fn name(&self) -> &str {
        self.data.name()
    }

    /// Get a mutable reference to the `NodeConfig`
    pub fn config_mut(&mut self) -> &mut NodeConfig {
        self.data.config_mut()
    }

    /// Get a reference to the `NodeConfig`
    pub fn config(&self) -> &NodeConfig {
        self.data.config()
    }

    /// Get the node's `NodeType`, which is more general than `NodeType`
    pub fn node_type(&self) -> NodeType {
        self.data.node_type()
    }

    /// Call the node's `ports()` function if it has one, returning the
    /// `PortsList` object
    pub fn provided_ports(&self) -> PortsList {
        self.node.ports()
    }

    /// Return an iterator over the children. Returns `None` if this node
    /// has no children (i.e. an `Action` node)
    pub fn children(&self) -> Option<&[TreeNode]> {
        if self.data.children.is_empty() {
            None
        } else {
            Some(&self.data.children)
        }
    }

    /// Return a mutable iterator over the children. Returns `None` if this node
    /// has no children (i.e. an `Action` node)
    pub fn children_mut(&mut self) -> Option<&mut [TreeNode]> {
        if self.data.children.is_empty() {
            None
        } else {
            Some(&mut self.data.children)
        }
    }
}

impl NodeDataGeneric {
    /// Returns the current node's status
    pub fn status(&self) -> NodeStatus {
        self.status
    }

    /// Sets the status of this node
    pub fn set_status(&mut self, status: NodeStatus) {
        self.status = status;
    }

    /// Resets the status back to `NodeStatus::Idle`
    pub fn reset_status(&mut self) {
        self.status = NodeStatus::Idle;
    }

    /// Get the name of the node
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a mutable reference to the `NodeConfig`
    pub fn config_mut(&mut self) -> &mut NodeConfig {
        &mut self.config
    }

    /// Get a reference to the `NodeConfig`
    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    /// Get the node's `NodeType`, which is more general than `NodeType`
    pub fn node_type(&self) -> NodeType {
        self.node_type
    }
}

// =============================
// Enum Definitions
// =============================

#[derive(Debug, Error)]
pub enum NodeError {
    #[error(
        "Child node of [{0}] returned invalid status [NodeStatus::{1}] when it is not allowed"
    )]
    StatusError(String, String),
    #[error("Out of bounds index")]
    IndexError,
    #[error("Couldn't find port [{0}]")]
    PortError(String),
    #[error("Couldn't parse port [{0}] value into specified type [{1}]")]
    /// # Arguments
    /// * Port name
    /// * Expected type
    PortValueParseError(String, String),
    #[error("Couldn't find entry in blackboard [{0}]")]
    BlackboardError(String),
    #[error("{0}")]
    UserError(#[from] anyhow::Error),
    #[error("{0}")]
    NodeStructureError(String),
    #[error("Decorator node does not have a child.")]
    ChildMissing,
    #[error("Blackboard lock was poisoned.")]
    LockPoisoned,
    #[error("A tick method was called that should have been unreachable. Please report this.")]
    UnreachableTick,
    #[error("Error evaluating a Condition expression: {0}")]
    ConditionExpressionError(String),
}

/// TODO: Not currently used
#[derive(Clone, Debug)]
pub enum PreCond {
    FailureIf,
    SuccessIf,
    SkipIf,
    WhileTrue,
    Count,
}

/// TODO: Not currently used
#[derive(Clone, Debug)]
pub enum PostCond {
    OnHalted,
    OnFailure,
    OnSuccess,
    Always,
    Count,
}

#[derive(Clone, Debug)]
pub enum NodeRuntime {
    Async,
    Sync,
    All,
}

// =========================================
// Struct Definitions and Implementations
// =========================================

/// Contains all common configuration that all types of nodes use.
#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub blackboard: Blackboard,
    pub input_ports: PortsRemapping,
    pub output_ports: PortsRemapping,
    pub manifest: Option<Arc<TreeNodeManifest>>,
    pub uid: u16,
    /// TODO: doesn't show actual path yet
    pub path: String,
    /// TODO: not used
    pub(crate) _pre_conditions: HashMap<PreCond, String>,
    /// TODO: not used
    pub(crate) _post_conditions: HashMap<PostCond, String>,
}

impl NodeConfig {
    pub fn new(blackboard: Blackboard) -> NodeConfig {
        Self {
            blackboard,
            input_ports: HashMap::new(),
            output_ports: HashMap::new(),
            manifest: None,
            uid: 1,
            path: String::from("TODO"),
            _pre_conditions: HashMap::new(),
            _post_conditions: HashMap::new(),
        }
    }

    /// Returns a reference to the blackboard.
    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    /// Adds a port to the config based on the direction. Used during XML parsing.
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
            PortDirection::Input => self.input_ports.contains_key(name),
            PortDirection::Output => self.output_ports.contains_key(name),
            _ => false,
        }
    }

    /// Returns a pointer to the `TreeNodeManifest` for this node.
    /// Only used during XML parsing.
    pub fn manifest(&self) -> Result<Arc<TreeNodeManifest>, ParseError> {
        match self.manifest.as_ref() {
            Some(manifest) => Ok(Arc::clone(manifest)),
            None => Err(ParseError::InternalError(
                "Missing manifest. This shouldn't happen; please report this.".to_string(),
            )),
        }
    }

    /// Replace the inner manifest.
    pub fn set_manifest(&mut self, manifest: Arc<TreeNodeManifest>) {
        let _ = self.manifest.insert(manifest);
    }

    /// Returns the value of the input port at the `port` key as a `Result<T, NodeError>`.
    /// The value is `Err` in the following situations:
    /// - The port wasn't found at that key
    /// - `T` doesn't match the type of the stored value
    /// - If a default value is needed (value is empty), couldn't parse default value
    /// - If a remapped key (e.g. a port value of `"{foo}"` references the blackboard
    ///     key `"foo"`), blackboard entry wasn't found or couldn't be read as `T`
    /// - If port value is a string, couldn't convert it to `T` using `parse_str()`.
    pub fn get_input<T>(&mut self, port: &str) -> Result<T, NodeError>
    where
        T: FromString + Clone + Send + 'static,
    {
        match self.input_ports.get(port) {
            Some(val) => {
                // Check if default is needed
                if val.is_empty() {
                    match self.manifest() {
                        Ok(manifest) => {
                            let port_info = manifest.ports.get(port).unwrap();
                            match port_info.default_value() {
                                Some(default) => match default.parse_str() {
                                    Ok(value) => Ok(value),
                                    Err(_) => Err(NodeError::PortError(String::from(port))),
                                },
                                None => Err(NodeError::PortError(String::from(port))),
                            }
                        }
                        Err(_) => Err(NodeError::PortError(String::from(port))),
                    }
                } else {
                    match get_remapped_key(port, val) {
                        // Value is a Blackboard pointer
                        Some(key) => match self.blackboard.get::<T>(&key) {
                            Some(val) => Ok(val),
                            None => Err(NodeError::BlackboardError(key)),
                        },
                        // Value is just a normal string
                        None => match <T as FromString>::from_string(val) {
                            Ok(val) => Ok(val),
                            Err(_) => Err(NodeError::PortValueParseError(
                                String::from(port),
                                format!("{:?}", TypeId::of::<T>()),
                            )),
                        },
                    }
                }
            }
            // Port not found
            None => Err(NodeError::PortError(String::from(port))),
        }
    }

    /// Sets `value` into the blackboard. The key is based on the value provided
    /// to the port at `port`.
    ///
    /// # Examples
    ///
    /// - Port value: `"="`: uses the port name as the blackboard key
    /// - `"foo"` uses `"foo"` as the blackboard key
    /// - `"{foo}"` uses `"foo"` as the blackboard key
    pub fn set_output<T>(&mut self, port: &str, value: T) -> Result<(), NodeError>
    where
        T: Clone + Send + 'static,
    {
        match self.output_ports.get(port) {
            Some(port_value) => {
                let blackboard_key = match port_value.as_str() {
                    "=" => port.to_string(),
                    value => match value.is_bb_pointer() {
                        true => value.strip_bb_pointer().unwrap(),
                        false => value.to_string(),
                    },
                };

                self.blackboard.set(blackboard_key, value);

                Ok(())
            }
            None => Err(NodeError::PortError(port.to_string())),
        }
    }
}

impl Clone for Box<dyn PortValue> {
    fn clone(&self) -> Box<dyn PortValue> {
        self.clone_port()
    }
}
