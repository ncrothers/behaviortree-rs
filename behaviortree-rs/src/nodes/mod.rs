use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use futures::future::BoxFuture;
use thiserror::Error;

use crate::{
    basic_types::{
        get_remapped_key, FromString, NodeCategory, ParseStr, PortDirection, PortValue,
        PortsRemapping, TreeNodeManifest,
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
type TickFn = for<'a> fn(
    &'a mut TreeNodeData,
    &'a mut Box<dyn Any + Send + Sync>,
) -> BoxFuture<'a, Result<NodeStatus, NodeError>>;
type HaltFn = for<'a> fn(&'a mut TreeNodeData, &'a mut Box<dyn Any + Send + Sync>) -> BoxFuture<'a, ()>;
type PortsFn = fn() -> PortsList;

#[derive(Clone, Copy, Debug)]
pub enum NodeType {
    Control,
    Decorator,
    StatefulAction,
    SyncAction,
}

#[derive(Debug)]
pub struct TreeNodeData {
    pub name: String,
    pub type_str: String,
    pub node_type: NodeType,
    pub node_category: NodeCategory,
    pub config: NodeConfig,
    pub status: NodeStatus,
    /// Vector of child nodes
    pub children: Vec<TreeNode>,
    pub ports_fn: PortsFn,
}

#[derive(Debug)]
pub struct TreeNode {
    pub data: TreeNodeData,
    pub context: Box<dyn Any + Send + Sync>,
    /// Function pointer to tick
    pub tick_fn: TickFn,
    /// Function pointer to on_start function (if StatefulActionNode)
    /// Otherwise points to tick_fn
    pub start_fn: TickFn,
    /// Function pointer to halt
    pub halt_fn: HaltFn,
}

impl TreeNode {
    /// Returns the current node's status
    pub fn status(&self) -> NodeStatus {
        self.data.status
    }

    /// Resets the status back to `NodeStatus::Idle`
    pub fn reset_status(&mut self) {
        self.data.status = NodeStatus::Idle;
    }

    /// Update the node's status
    pub fn set_status(&mut self, status: NodeStatus) {
        self.data.status = status;
    }

    /// Internal-only, calls the action-type-specific tick
    async fn action_tick(&mut self) -> NodeResult {
        match self.data.node_type {
            NodeType::StatefulAction => {
                let prev_status = self.data.status;

                let new_status = match prev_status {
                    NodeStatus::Idle => {
                        ::log::debug!("[behaviortree_rs]: {}::on_start()", &self.data.config.path);
                        // let mut wrapper = ArgWrapper::new(&mut self.data, &mut self.context);
                        let new_status = (self.start_fn)(&mut self.data, &mut self.context).await?;
                        // drop(wrapper);
                        if matches!(new_status, NodeStatus::Idle) {
                            return Err(NodeError::StatusError(
                                format!("{}::on_start()", self.data.config.path),
                                "Idle".to_string(),
                            ));
                        }
                        new_status
                    }
                    NodeStatus::Running => {
                        ::log::debug!(
                            "[behaviortree_rs]: {}::on_running()",
                            &self.data.config.path
                        );
                        let new_status = (self.tick_fn)(&mut self.data, &mut self.context).await?;
                        if matches!(new_status, NodeStatus::Idle) {
                            return Err(NodeError::StatusError(
                                format!("{}::on_running()", self.data.config.path),
                                "Idle".to_string(),
                            ));
                        }
                        new_status
                    }
                    prev_status => prev_status,
                };

                self.set_status(new_status);

                Ok(new_status)
            }
            NodeType::SyncAction => {
                match (self.tick_fn)(&mut self.data, &mut self.context).await? {
                    status @ (NodeStatus::Running | NodeStatus::Idle) => {
                        Err(::behaviortree_rs::nodes::NodeError::StatusError(
                            self.data.config.path.clone(),
                            status.to_string(),
                        ))
                    }
                    status => Ok(status),
                }
            }
            _ => panic!(
                "This should not be possible, action_tick() was called for a non-action node"
            ),
        }
    }

    /// Tick the node
    pub async fn execute_tick(&mut self) -> NodeResult {
        match self.data.node_type {
            NodeType::Control | NodeType::Decorator => {
                (self.tick_fn)(&mut self.data, &mut self.context).await
            }
            NodeType::StatefulAction | NodeType::SyncAction => self.action_tick().await,
        }
    }

    /// Halt the node
    pub async fn halt(&mut self) {
        (self.halt_fn)(&mut self.data, &mut self.context).await;
    }

    /// Get the name of the node
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// Get a mutable reference to the `NodeConfig`
    pub fn config_mut(&mut self) -> &mut NodeConfig {
        &mut self.data.config
    }

    /// Get a reference to the `NodeConfig`
    pub fn config(&self) -> &NodeConfig {
        &self.data.config
    }

    /// Get the node's `NodeType`, which is only:
    /// * `NodeType::Control`
    /// * `NodeType::Decorator`
    /// * `NodeType::SyncAction`
    /// * `NodeType::StatefulAction`
    pub fn node_type(&self) -> NodeType {
        self.data.node_type
    }

    /// Get the node's `NodeCategory`, which is more general than `NodeType`
    pub fn node_category(&self) -> NodeCategory {
        self.data.node_category
    }

    /// Call the node's `ports()` function if it has one, returning the
    /// `PortsList` object
    pub fn provided_ports(&self) -> PortsList {
        (self.data.ports_fn)()
    }

    /// Return an iterator over the children. Returns `None` if this node
    /// has no children (i.e. an `Action` node)
    pub fn children(&self) -> Option<impl Iterator<Item = &TreeNode>> {
        if self.data.children.is_empty() {
            None
        } else {
            Some(self.data.children.iter())
        }
    }

    /// Return a mutable iterator over the children. Returns `None` if this node
    /// has no children (i.e. an `Action` node)
    pub fn children_mut(&mut self) -> Option<impl Iterator<Item = &mut TreeNode>> {
        if self.data.children.is_empty() {
            None
        } else {
            Some(self.data.children.iter_mut())
        }
    }
}

impl TreeNodeData {
    /// Halt children from this index to the end.
    ///
    /// # Errors
    ///
    /// Returns `NodeError::IndexError` if `start` is out of bounds.
    pub async fn halt_children(&mut self, start: usize) -> NodeResult<()> {
        if start >= self.children.len() {
            return Err(NodeError::IndexError);
        }

        let end = self.children.len();

        for i in start..end {
            self.halt_child_idx(i).await?;
        }

        Ok(())
    }

    /// Halts and resets all children
    pub async fn reset_children(&mut self) {
        self.halt_children(0)
            .await
            .expect("reset_children failed, shouldn't be possible. Report this")
    }

    /// Halt child at the `index`. Not to be confused with `halt_child()`, which is
    /// a helper that calls `halt_child_idx(0)`, primarily used for `Decorator` nodes.
    pub async fn halt_child_idx(&mut self, index: usize) -> NodeResult<()> {
        let child = self.children.get_mut(index).ok_or(NodeError::IndexError)?;
        if child.status() == ::behaviortree_rs::nodes::NodeStatus::Running {
            child.halt().await;
        }
        child.reset_status();
        Ok(())
    }

    /// Sets the status of this node
    pub fn set_status(&mut self, status: NodeStatus) {
        self.status = status;
    }

    /// Calls `halt_child_idx(0)`. This should only be used in
    /// `Decorator` nodes
    pub async fn halt_child(&mut self) {
        self.reset_child().await
    }

    /// Halts and resets the first child. This should only be used in
    /// `Decorator` nodes
    pub async fn reset_child(&mut self) {
        if let Some(child) = self.children.get_mut(0) {
            if matches!(child.status(), NodeStatus::Running) {
                child.halt().await;
            }

            child.reset_status();
        }
    }

    /// Gets a mutable reference to the first child. Helper for
    /// `Decorator` nodes to get their child.
    pub fn child(&mut self) -> Option<&mut TreeNode> {
        self.children.get_mut(0)
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

                self.blackboard.set(blackboard_key, value);

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
