use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use quick_xml::events::attributes::Attributes;
use thiserror::Error;

use crate::{blackboard::BlackboardString, error::ParseError};

/// Specifies all types of nodes that can be used in a behavior tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    /// Leaf node that executes an action
    Action,
    /// Node with children that executes a certain child based on a condition
    Condition,
    /// Node with multiple children that executes them in some way.
    /// Examples like `Sequence`, `Parallel`.
    Control,
    /// Node with one child that modifies the execution or result of the node.
    Decorator,
    /// Leaf node that is a reference to another BehaviorTree.
    SubTree,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Action => "Action",
            Self::Condition => "Condition",
            Self::Control => "Control",
            Self::Decorator => "Decorator",
            Self::SubTree => "SubTree",
        };

        write!(f, "{text}")
    }
}

/// Specifies the status of a node's execution. Returned from
/// functions `execute_tick()` and `tick()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStatus {
    Idle,
    Running,
    Success,
    Failure,
    Skipped,
}

impl NodeStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Idle | Self::Skipped)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Success | Self::Failure)
    }

    pub fn to_colorized_string(&self) -> String {
        let color_start = match self {
            Self::Idle => "\x1b[36m",
            Self::Running => "\x1b[33m",
            Self::Success => "\x1b[32m",
            Self::Failure => "\x1b[31m",
            Self::Skipped => "\x1b[34m",
        };

        color_start.to_string() + &self.to_string() + "\x1b[0m"
    }
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Idle => "IDLE",
            Self::Running => "RUNNING",
            Self::Success => "SUCCESS",
            Self::Failure => "FAILURE",
            Self::Skipped => "SKIPPED",
        };

        write!(f, "{text}")
    }
}

#[derive(Error, Debug)]
pub enum ParseNodeStatusError {
    #[error("string didn't match any NodeStatus values")]
    NoMatch,
}

#[derive(Error, Debug)]
pub enum ParseNodeTypeError {
    #[error("string didn't match any NodeType values")]
    NoMatch,
}

#[derive(Error, Debug)]
pub enum ParsePortDirectionError {
    #[error("string didn't match any PortDirection values")]
    NoMatch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortDirection {
    Input,
    Output,
    InOut,
}

impl std::fmt::Display for PortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Input => "Input",
            Self::Output => "Output",
            Self::InOut => "InOut",
        };

        write!(f, "{text}")
    }
}

// ===========================
// Converting string to types
// ===========================

impl FromStr for NodeStatus {
    type Err = ParseNodeStatusError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "IDLE" | "Idle" => Ok(NodeStatus::Idle),
            "RUNNING" | "Running" => Ok(NodeStatus::Running),
            "SUCCESS" | "Success" => Ok(NodeStatus::Success),
            "FAILURE" | "Failure" => Ok(NodeStatus::Failure),
            "SKIPPED" | "Skipped" => Ok(NodeStatus::Skipped),
            _ => Err(ParseNodeStatusError::NoMatch),
        }
    }
}

impl FromStr for NodeType {
    type Err = ParseNodeTypeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Action" => Ok(NodeType::Action),
            "Condition" => Ok(NodeType::Condition),
            "Control" => Ok(NodeType::Control),
            "Decorator" => Ok(NodeType::Decorator),
            "SubTree" => Ok(NodeType::SubTree),
            _ => Err(ParseNodeTypeError::NoMatch),
        }
    }
}

impl FromStr for PortDirection {
    type Err = ParsePortDirectionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Input" | "INPUT" => Ok(PortDirection::Input),
            "Output" | "OUTPUT" => Ok(PortDirection::Output),
            "InOut" | "INOUT" => Ok(PortDirection::InOut),
            _ => Err(ParsePortDirectionError::NoMatch),
        }
    }
}

// ===========================
// End of String Conversions
// ===========================

/// Wrapper around a `HashMap` storing ports
#[derive(Debug, Default, PartialEq)]
pub struct PortsList(HashMap<String, PortInfo>);

impl<T> From<T> for PortsList
where
    T: IntoIterator<Item = PortInfo>,
{
    fn from(value: T) -> Self {
        let list = value
            .into_iter()
            .map(|info| (info.name.clone(), info))
            .collect();

        Self(list)
    }
}

impl Deref for PortsList {
    type Target = HashMap<String, PortInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PortsList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Data pertaining to the node at the time of instantiation during parsing
#[derive(Debug)]
pub struct TreeNodeManifest {
    pub node_type: NodeType,
    pub registration_id: String,
    pub ports: PortsList,
    pub description: String,
}

impl TreeNodeManifest {
    pub fn new(
        node_type: NodeType,
        registration_id: String,
        ports: PortsList,
        description: String,
    ) -> TreeNodeManifest {
        Self {
            node_type,
            registration_id,
            ports,
            description,
        }
    }
}

// ===========================
// Ports
// ===========================

/// Returns whether `name` is a valid node port identifier
pub(crate) fn is_allowed_port_name(name: &str) -> bool {
    if name.is_empty() {
        false
    } else if name == "_autoremap" {
        true
    } else if !name.chars().next().unwrap().is_ascii_alphabetic() {
        false
    } else {
        // If the name isn't name or ID, it's valid
        !(name == "name" || name == "ID")
    }
}

pub struct PortInfoBuilder<T> {
    name: String,
    direction: PortDirection,
    description: Option<String>,
    default_value: Option<Box<dyn Any + Send + Sync>>,
    #[cfg(feature = "expr")]
    parse_expr: bool,
    type_id: TypeId,
    _pd: PhantomData<T>,
}

/// Metadata about a node port
#[derive(Debug)]
pub struct PortInfo {
    name: String,
    /// Direction category for the port
    direction: PortDirection,
    type_id: TypeId,
    /// Optional description of the port
    description: String,
    default_value: Option<Box<dyn Any + Send + Sync>>,
    /// When `true`, should parse the port value as an expression when loading
    /// the tree to validate syntax.
    #[cfg(feature = "expr")]
    parse_expr: bool,
}

impl<T> PortInfoBuilder<T>
where
    T: Any + Send + Sync + 'static,
{
    pub fn build(self) -> PortInfo {
        PortInfo {
            name: self.name,
            direction: self.direction,
            type_id: self.type_id,
            description: self.description.unwrap_or_default(),
            default_value: self.default_value,
            #[cfg(feature = "expr")]
            parse_expr: self.parse_expr,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());

        self
    }

    pub fn default_value(mut self, value: impl Into<T>) -> Self {
        self.default_value = Some(Box::new(value.into()));

        self
    }

    #[cfg(feature = "expr")]
    pub fn parse_expr(mut self) -> Self {
        self.parse_expr = true;

        self
    }
}

impl PortInfo {
    /// Start a builder for an input port. The type `T` must be specified, and
    /// this type will be enforced when retrieving the value later with
    /// `get_input()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let port = PortInfo::input::<i32>("foo")
    ///     // Optionally, set the default value of the port
    ///     .default_value(10i32)
    ///     // Optionally, set the description for the port
    ///     .description("Port description")
    ///     // If the `expr` feature is enabled, optionally enable expression
    ///     // validation during parsing.
    ///     // .parse_expr()
    ///     // Finally, build and get the `PortInfo`
    ///     .build();
    ///
    /// assert!(port.has_default());
    /// assert_eq!(port.default_value::<i32>().copied(), Some(10));
    /// ```
    pub fn input<T: Any + Send + Sync + 'static>(name: impl Into<String>) -> PortInfoBuilder<T> {
        PortInfoBuilder {
            name: name.into(),
            direction: PortDirection::Input,
            type_id: TypeId::of::<T>(),
            description: None,
            default_value: None,
            #[cfg(feature = "expr")]
            parse_expr: false,
            _pd: PhantomData,
        }
    }

    pub fn output(name: impl Into<String>) -> PortInfoBuilder<String> {
        PortInfoBuilder {
            name: name.into(),
            direction: PortDirection::Output,
            type_id: TypeId::of::<String>(),
            description: None,
            default_value: None,
            #[cfg(feature = "expr")]
            parse_expr: false,
            _pd: PhantomData::<String>,
        }
    }

    pub fn has_default(&self) -> bool {
        self.default_value.is_some()
    }

    // TODO: Make this function (and others like it) return `Result` instead of `Option`?
    pub fn default_value<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        // Check the type IDs are the same first
        if TypeId::of::<T>() != self.type_id {
            None
        } else {
            self.default_value
                .as_ref()
                .and_then(|val| val.downcast_ref())
        }
    }

    #[cfg(feature = "expr")]
    pub fn parse_expr(&self) -> bool {
        self.parse_expr
    }

    pub fn direction(&self) -> PortDirection {
        self.direction
    }
}

impl PartialEq for PortInfo {
    #[allow(unused_mut, unused_assignments)]
    fn eq(&self, other: &Self) -> bool {
        // Does not check equality between default values
        let mut base_eq = self.name == other.name
            && self.direction == other.direction
            && self.type_id == other.type_id
            && self.description == other.description;

        #[cfg(feature = "expr")]
        {
            // Only check this on the expr feature
            base_eq = self.parse_expr == other.parse_expr;
        }

        base_eq
    }
}

/// Remap a blackboard key
pub(crate) fn get_remapped_key(port_name: &str, remapped_port: &str) -> Option<String> {
    // When the port value is "=", use the port name
    if remapped_port == "=" || remapped_port == "{=}" {
        Some(port_name.to_string())
    } else {
        remapped_port.strip_bb_pointer().map(ToString::to_string)
    }
}

// ===========================
// Private Helpers
// ===========================

/// Helper trait for parsing XML
pub(crate) trait AttrsToMap {
    fn to_map(self) -> Result<HashMap<String, String>, ParseError>;
}

impl AttrsToMap for Attributes<'_> {
    fn to_map(self) -> Result<HashMap<String, String>, ParseError> {
        let mut map = HashMap::new();
        for attr in self.into_iter() {
            let attr = attr?;
            let name = String::from_utf8(attr.key.0.into())?;
            let value = String::from_utf8(attr.value.to_vec())?;

            map.insert(name, value);
        }

        Ok(map)
    }
}
