pub mod action;
pub use action::*;
pub mod control;
pub use control::*;
pub mod decorator;
pub use decorator::*;
pub mod error;
pub use error::*;

use std::{
    any::TypeId,
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::Arc,
};

use crate::{
    basic_types::{get_remapped_key, NodeType, PortDirection, TreeNodeManifest},
    blackboard::BlackboardString,
    Blackboard,
};

pub use crate::basic_types::{NodeStatus, PortsList};

pub type NodeResult<Output = NodeStatus> = Result<Output, NodeError>;

/// Trait defining the basic methods common to all nodes. When traversing the
/// tree and/or accessing a node's children, these are the methods that are
/// available.
pub trait NodeBase: std::fmt::Debug + Send + Sync {
    fn ports(&self) -> PortsList;
    fn execute_tick(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult;
    fn halt(&mut self, ctx: &mut NodeDataGeneric) -> NodeResult<()>;
}

/// Trait with a blanket implementation for each node-type-specific trait.
/// You shouldn't need to implement this yourself; it should already be implemented
/// for nodes that you create.
///
/// The method wraps your node into a generic `Box<dyn NodeBase>`.
///
/// # Example
///
/// ```
/// use behaviortree_rs::prelude::*;
///
/// #[derive(Debug)]
/// struct MyNode {
///     foo: i32,
/// }
///
/// impl SyncActionNode for MyNode {
///     fn ports(&self) -> PortsList {
///         PortsList::from([
///             PortInfo::input::<i32>("foo").build()
///         ])
///     }
///
///     fn tick(&mut self, _ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
///         Ok(NodeStatus::Success)
///     }
/// }
///
/// let node: Box<dyn NodeBase> = MyNode { foo: 10 }.to_boxed();
///
/// let expected_ports = PortsList::from([
///     PortInfo::input::<i32>("foo").build()
/// ]);
///
/// let ports = node.ports();
///
/// assert_eq!(ports, expected_ports);
/// ```
pub trait ToBoxed<T> {
    fn to_boxed(self) -> Box<dyn NodeBase>;
}

/// Data common to all nodes, without any additional helper methods for node-type-specific
/// functionality, like is available when wrapped in [`NodeData<T>`].
#[derive(Debug)]
pub struct NodeDataGeneric {
    /// Metadata fields that are less-commonly referenced
    pub(crate) meta: NodeMetadata,
    /// Current [`NodeStatus`] of this node
    pub(crate) status: NodeStatus,
    /// Vector of child nodes
    pub children: Vec<TreeNode>,
    /// The [`Blackboard`] belonging to this node, which is linked to the
    /// subtree this node belongs to (if applicable)
    pub blackboard: Blackboard,
}

/// Provides access to a node's common data and configuration. This struct is
/// passed into node methods such as `tick()`, `halt()`, etc.
///
/// The generic parameter is used to restrict/expand access to helper methods
/// based on the type of node. For example, leaf nodes have no children, so
/// they don't need (and shouldn't have access to) helper methods related to
/// ticking or halting children.
pub struct NodeData<'a, T> {
    data: &'a mut NodeDataGeneric,
    _pd: PhantomData<T>,
}

impl<T> Deref for NodeData<'_, T> {
    type Target = NodeDataGeneric;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> DerefMut for NodeData<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, T> NodeData<'a, T> {
    pub fn new(data: &'a mut NodeDataGeneric) -> Self {
        Self {
            data,
            _pd: PhantomData,
        }
    }
}

/// Node within a [`Tree`](crate::prelude::Tree). Can contain any number of children
/// based on the underlying [`NodeType`], which are held in the `data` field.
///
/// # Ticking
///
/// To tick this node, call `execute_tick()`. This will call the underlying
/// node implementation of [`NodeBase`]'s `execute_tick()` method, which may
/// propagate down the tree.
#[derive(Debug)]
pub struct TreeNode {
    /// Data common to all [`TreeNode`]s.
    pub data: NodeDataGeneric,
    /// Trait object for the implementation of this node.
    pub(crate) node: Box<dyn NodeBase>,
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
        let status = self.node.execute_tick(&mut self.data)?;

        // Preserve Idle state if skipped, but communicate Skipped to the parent
        if !matches!(status, NodeStatus::Skipped) {
            self.data.set_status(status);
        }

        Ok(status)
    }

    /// Halt the node
    pub fn halt(&mut self) -> NodeResult<()> {
        self.node.halt(&mut self.data)
    }

    /// Get the name of the node
    pub fn name(&self) -> &str {
        self.data.name()
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

    /// Return a slice over the children. Returns `None` if this node
    /// has no children (i.e. an `Action` node)
    pub fn children(&self) -> Option<&[TreeNode]> {
        if self.data.children.is_empty() {
            None
        } else {
            Some(&self.data.children)
        }
    }

    /// Return a mutable slice over the children. Returns `None` if this node
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
    /// Get the name of the node
    pub fn name(&self) -> &str {
        &self.meta.name
    }

    /// Get the path of the in the tree
    pub fn path(&self) -> &str {
        &self.meta.path
    }

    /// Returns a reference to the blackboard.
    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    /// Returns the current node's status
    pub fn status(&self) -> NodeStatus {
        self.status
    }

    /// Sets the status of this node
    pub fn set_status(&mut self, status: NodeStatus) {
        self.status = status;
    }

    /// Resets the status back to [`NodeStatus::Idle`]
    pub fn reset_status(&mut self) {
        self.status = NodeStatus::Idle;
    }

    /// Get a mutable reference to the [`NodeMetadata`]
    pub(crate) fn metadata_mut(&mut self) -> &mut NodeMetadata {
        &mut self.meta
    }

    /// Get a reference to the [`NodeMetadata`]
    pub(crate) fn metadata(&self) -> &NodeMetadata {
        &self.meta
    }

    /// Get the node's [`NodeType`]
    pub fn node_type(&self) -> NodeType {
        self.meta.node_type
    }

    /// Get the node's [`TreeNodeManifest`], which contains some metadata defined
    /// when the node was registered.
    pub fn manifest(&self) -> &TreeNodeManifest {
        &self.meta.manifest
    }

    /// Returns the value of the input port at the `port` key as a `Result<T, NodeError>`.
    /// The value is `Err` in the following situations:
    /// - The port wasn't found at that key
    /// - `T` doesn't match the type of the stored value
    /// - If a default value is needed (value is empty), couldn't parse default value
    /// - If a remapped key (e.g. a port value of `"{foo}"` references the blackboard
    ///     key `"foo"`), blackboard entry wasn't found or couldn't be read as `T`
    /// - If port value is a string, couldn't convert it to `T` using `parse_str()`.
    pub fn get_input<T>(&self, port_name: &str) -> Result<T, NodeError>
    where
        T: FromStr + Clone + Send + 'static,
    {
        // Check if port exists first
        if !self.meta.manifest.ports.contains_key(port_name) {
            return Err(NodeError::PortError(port_name.to_string()));
        }

        match self.meta.port_values.get(port_name) {
            Some((PortDirection::Input, val)) => {
                match get_remapped_key(port_name, val) {
                    // Value is a Blackboard pointer
                    Some(key) => match self.blackboard.get::<T>(&key) {
                        Some(val) => Ok(val.clone()),
                        None => Err(NodeError::BlackboardError(key)),
                    },
                    // Value is just a normal string
                    None => match <T as FromStr>::from_str(val) {
                        Ok(val) => Ok(val),
                        Err(_) => Err(NodeError::PortValueParseError(
                            String::from(port_name),
                            format!("{:?}", TypeId::of::<T>()),
                        )),
                    },
                }
            }
            // Return error if it's not an input port
            Some((dir, _)) => Err(NodeError::PortDirectionError {
                name: port_name.to_string(),
                actual: *dir,
                expected: PortDirection::Input,
            }),
            // Try to load default since a value wasn't set in the XML
            None => {
                // Unwrapping is safe because we already verified the port exists
                let port_info = self.meta.manifest.ports.get(port_name).unwrap();

                // Return error if it's not an input port
                if port_info.direction() != PortDirection::Input {
                    return Err(NodeError::PortDirectionError {
                        name: port_name.to_string(),
                        actual: port_info.direction(),
                        expected: PortDirection::Input,
                    });
                }

                // Return error if there's no default
                if !port_info.has_default() {
                    return Err(NodeError::MissingRequiredPort(port_name.to_string()));
                }

                match port_info.default_value::<T>() {
                    Some(default) => Ok(default.clone()),
                    // The only reason this returns `None` is if the types aren't the same
                    None => Err(NodeError::PortTypeMismatch(String::from(port_name))),
                }
            }
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
    pub fn set_output<T>(&self, port_name: &str, value: T) -> Result<(), NodeError>
    where
        T: Send + 'static,
    {
        match self.meta.port_values.get(port_name) {
            // Only match if port exists and is an output port
            Some((PortDirection::Output, port_value)) => {
                let blackboard_key = match port_value.as_str() {
                    "=" => port_name.to_string(),
                    value => match value.is_bb_pointer() {
                        true => value.strip_bb_pointer().unwrap().to_string(),
                        false => value.to_string(),
                    },
                };

                self.blackboard.set(blackboard_key, value);

                Ok(())
            }
            _ => Err(NodeError::PortError(port_name.to_string())),
        }
    }
}

// =============================
// Enum Definitions
// =============================

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

// =========================================
// Struct Definitions and Implementations
// =========================================

/// Contains all common configuration that all types of nodes use.
#[derive(Clone, Debug)]
pub(crate) struct NodeMetadata {
    pub(crate) uid: u16,
    /// Name of the node as registered in the `Factory`
    pub(crate) name: String,
    /// TODO: doesn't show actual path yet
    pub(crate) path: String,
    /// The type of this node
    pub(crate) node_type: NodeType,
    /// Values of ports set in the node XML attributes
    pub(crate) port_values: HashMap<String, (PortDirection, String)>,
    pub(crate) manifest: Arc<TreeNodeManifest>,
    /// TODO: not used
    pub(crate) _pre_conditions: HashMap<PreCond, String>,
    /// TODO: not used
    pub(crate) _post_conditions: HashMap<PostCond, String>,
}

impl NodeMetadata {
    pub(crate) fn new(
        name: String,
        path: String,
        node_type: NodeType,
        manifest: Arc<TreeNodeManifest>,
    ) -> Self {
        Self {
            uid: 0,
            name,
            path,
            node_type,
            port_values: HashMap::new(),
            manifest,
            _pre_conditions: HashMap::new(),
            _post_conditions: HashMap::new(),
        }
    }

    /// Sets the value of a port. Used in XML parsing to set the value of a port
    /// using the string value of the XML attribute.
    pub(crate) fn set_port_value(&mut self, direction: PortDirection, name: String, value: String) {
        self.port_values.insert(name, (direction, value));
    }

    /// Returns whether the value for `name` and `direction` has been set using
    /// [`NodeMetadata::set_port_value`].
    pub(crate) fn is_port_value_set(&self, name: &str, direction: PortDirection) -> bool {
        self.port_values
            .get(name)
            .map(|(dir, _)| *dir == direction)
            .unwrap_or(false)
    }
}
