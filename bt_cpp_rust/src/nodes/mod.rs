use std::{any::TypeId, collections::HashMap, sync::Arc};

use futures::future::BoxFuture;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{
    basic_types::{
        self, get_remapped_key, BTToString, FromString, ParseStr, PortDirection, PortValue,
        PortsRemapping, TreeNodeManifest,
    },
    blackboard::BlackboardString,
    tree::ParseError,
    Blackboard,
};

pub use crate::basic_types::{NodeStatus, PortsList};

pub mod control;
pub use control::{ControlNode, ControlNodeBase, ControlNodePtr};

pub mod decorator;
pub use decorator::{DecoratorNode, DecoratorNodeBase, DecoratorNodePtr};

pub mod action;
pub use action::*;

// =============================
// Trait Definitions
// =============================

/// Supertrait that requires all of the base functions that need to
/// be implemented for every tree node.
pub trait TreeNodeBase:
    std::fmt::Debug
    + NodePorts
    + TreeNodeDefaults
    + GetNodeType
    + ExecuteTick
    + SyncHalt
    + AsyncHalt
    + SyncTick
    + AsyncTick
{
}

/// Pointer to the most general trait, which encapsulates all
/// node types that implement `TreeNodeBase` (all nodes need
/// to for it to compile)
pub type TreeNodePtr = Arc<Mutex<dyn TreeNodeBase + Send>>;

pub type NodeResult = Result<NodeStatus, NodeError>;

/// The only trait from `TreeNodeBase` that _needs_ to be
/// implemented manually, without a derive macro. This is where
/// the `tick()` is defined as well as the ports, with
/// `provided_ports()`.
pub trait NodePorts {
    fn provided_ports(&self) -> PortsList {
        HashMap::new()
    }
}

/// The only trait from `TreeNodeBase` that _needs_ to be
/// implemented manually, without a derive macro. This is where
/// the `tick()` is defined as well as the ports, with
/// `provided_ports()`.
pub trait SyncTick {
    fn tick(&mut self) -> NodeResult;
}

/// The only trait from `TreeNodeBase` that _needs_ to be
/// implemented manually, without a derive macro. This is where
/// the `tick()` is defined as well as the ports, with
/// `provided_ports()`.
pub trait AsyncTick {
    fn tick(&mut self) -> BoxFuture<NodeResult>;
}

/// Trait that defines the `halt()` function, which gets called
/// when a node is stopped. This function typically contains any
/// cleanup code for the node.
pub trait SyncHalt {
    fn halt(&mut self) {}
}

/// Trait that defines the `halt()` function, which gets called
/// when a node is stopped. This function typically contains any
/// cleanup code for the node.
pub trait AsyncHalt {
    fn halt(&mut self) -> BoxFuture<()> {
        Box::pin(async move {})
    }
}

pub trait RuntimeType {
    fn get_runtime(&self) -> NodeRuntime;
}

/// Trait that should only be implemented with a derive macro.
/// The automatic implementation defines helper functions.
///
/// The automatic implementation relies on certain named fields
/// within the struct that it gets derived on.
///
/// # Examples
///
/// The struct below won't compile, but it contains the base derived
/// traits and struct fields needed for all node definitions.
///
/// ```ignore
/// use bt_cpp_rust::basic_types::NodeStatus;
/// use bt_cpp_rust::nodes::NodeConfig;
/// use bt_cpp_rust::derive::TreeNodeDefaults;
///
/// #[derive(Debug, Clone, TreeNodeDefaults)]
/// struct MyTreeNode {
///     config: NodeConfig,
///     status: NodeStatus,
/// }
/// ```
pub trait TreeNodeDefaults {
    fn name(&self) -> &String;
    fn path(&self) -> &String;
    fn status(&self) -> NodeStatus;
    fn reset_status(&mut self);
    fn set_status(&mut self, status: NodeStatus);
    fn config(&mut self) -> &mut NodeConfig;
    fn into_boxed(self) -> Box<dyn TreeNodeBase>;
    fn to_tree_node_ptr(&self) -> TreeNodePtr;
    fn clone_node_boxed(&self) -> Box<dyn TreeNodeBase + Send + Sync>;
}

/// Automatically implemented for all node types. The implementation
/// differs based on the `NodeType`.
pub trait ExecuteTick {
    fn execute_tick(&mut self) -> BoxFuture<NodeResult>;
}

/// TODO
pub trait ConditionNode {}

/// Automatically implemented for all node types.
pub trait GetNodeType {
    fn node_type(&self) -> basic_types::NodeType;
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
    _pre_conditions: HashMap<PreCond, String>,
    /// TODO: not used
    _post_conditions: HashMap<PostCond, String>,
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
    /// key `"foo"`), blackboard entry wasn't found or couldn't be read as `T`
    /// - If port value is a string, couldn't convert it to `T` using `parse_str()`.
    pub async fn get_input<T>(&mut self, port: &str) -> Result<T, NodeError>
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
                        Some(key) => match self.blackboard.get::<T>(&key).await {
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

    /// Sync version of `get_input<T>`
    ///
    /// Returns the value of the input port at the `port` key as a `Result<T, NodeError>`.
    /// The value is `Err` in the following situations:
    /// - The port wasn't found at that key
    /// - `T` doesn't match the type of the stored value
    /// - If a default value is needed (value is empty), couldn't parse default value
    /// - If a remapped key (e.g. a port value of `"{foo}"` references the blackboard
    /// key `"foo"`), blackboard entry wasn't found or couldn't be read as `T`
    /// - If port value is a string, couldn't convert it to `T` using `parse_str()`.
    pub fn get_input_sync<T>(&mut self, port: &str) -> Result<T, NodeError>
    where
        T: FromString + Clone + Send + 'static,
    {
        futures::executor::block_on(self.get_input(port))
    }

    /// Sets `value` into the blackboard. The key is based on the value provided
    /// to the port at `port`.
    ///
    /// # Examples
    ///
    /// - Port value: `"="`: uses the port name as the blackboard key
    /// - `"foo"` uses `"foo"` as the blackboard key
    /// - `"{foo}"` uses `"foo"` as the blackboard key
    pub async fn set_output<T>(&mut self, port: &str, value: T) -> Result<(), NodeError>
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

                self.blackboard.set(blackboard_key, value).await;

                Ok(())
            }
            None => Err(NodeError::PortError(port.to_string())),
        }
    }

    /// Sync version of `set_output<T>`
    ///
    /// Sets `value` into the blackboard. The key is based on the value provided
    /// to the port at `port`.
    ///
    /// # Examples
    ///
    /// - Port value: `"="`: uses the port name as the blackboard key
    /// - `"foo"` uses `"foo"` as the blackboard key
    /// - `"{foo}"` uses `"foo"` as the blackboard key
    pub async fn set_output_sync<T>(&mut self, port: &str, value: T) -> Result<(), NodeError>
    where
        T: Clone + Send + 'static,
    {
        futures::executor::block_on(self.set_output(port, value))
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

impl Clone for Box<dyn TreeNodeBase + Send + Sync> {
    fn clone(&self) -> Box<dyn TreeNodeBase + Send + Sync> {
        self.clone_node_boxed()
    }
}
