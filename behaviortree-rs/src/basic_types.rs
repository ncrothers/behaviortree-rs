use std::{
    any::{Any, TypeId},
    collections::HashMap,
    convert::Infallible,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use quick_xml::events::attributes::Attributes;
use thiserror::Error;

use crate::{
    blackboard::BlackboardString,
    error::{ParseBoolError, ParseError},
    macros::{impl_from_string, impl_into_string},
};

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

    pub fn into_string_color(&self) -> String {
        let color_start = match self {
            Self::Idle => "\x1b[36m",
            Self::Running => "\x1b[33m",
            Self::Success => "\x1b[32m",
            Self::Failure => "\x1b[31m",
            Self::Skipped => "\x1b[34m",
        };

        color_start.to_string() + &self.bt_to_string() + "\x1b[0m"
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

///
/// Trait for custom conversion from String
///
/// Out of the box, `ParseStr<T>` is implemented on all numeric types, `bool`,
/// `NodeStatus`, `NodeType`, and `PortDirection`, and `Vec`s holding those types.
///
/// To implement `ParseStr<T>` on your own type, you can derive
/// the `behaviortree_rs` trait: `FromString` on it. To derive this
/// trait you will need to implement the Rust built-in trait `FromStr`.
/// You can also just implement `FromString` yourself, but it's recommended
/// to implement `FromStr` that also provides the `::parse()` function.
///
/// # Example
///
/// ```
/// use behaviortree_rs::derive::FromString;
///
/// #[derive(FromString)]
/// struct MyType {
///     foo: String
/// }
///
/// impl std::str::FromStr for MyType {
///     // Replace with your error
///     type Err = core::convert::Infallible;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         todo!()
///     }
/// }
///
/// ```
pub trait ParseStr<T> {
    type Err;

    fn parse_str(&self) -> Result<T, Self::Err>;
}

// Implements ParseStr<T> for all T that implements FromString
impl<T, U> ParseStr<T> for U
where
    T: FromString,
    U: AsRef<str>,
{
    type Err = <T as FromString>::Err;

    fn parse_str(&self) -> Result<T, Self::Err> {
        <T as FromString>::from_string(self)
    }
}

pub trait FromString
where
    Self: Sized,
{
    type Err;

    fn from_string(value: impl AsRef<str>) -> Result<Self, Self::Err>;
}

impl<T> FromString for Vec<T>
where
    T: FromString,
{
    type Err = <T as FromString>::Err;

    fn from_string(value: impl AsRef<str>) -> Result<Vec<T>, Self::Err> {
        value
            .as_ref()
            .split(';')
            .map(|x| T::from_string(x))
            .collect()
    }
}

impl_from_string!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

impl FromString for String {
    type Err = Infallible;

    fn from_string(value: impl AsRef<str>) -> Result<String, Self::Err> {
        Ok(value.as_ref().to_string())
    }
}

impl FromString for bool {
    type Err = ParseBoolError;

    fn from_string(value: impl AsRef<str>) -> Result<bool, ParseBoolError> {
        match value.as_ref() {
            "1" | "true" | "TRUE" => Ok(true),
            "0" | "false" | "FALSE" => Ok(false),
            _ => Err(ParseBoolError::ParseError),
        }
    }
}

impl FromString for NodeStatus {
    type Err = ParseNodeStatusError;

    fn from_string(value: impl AsRef<str>) -> Result<NodeStatus, Self::Err> {
        match value.as_ref() {
            "IDLE" | "Idle" => Ok(NodeStatus::Idle),
            "RUNNING" | "Running" => Ok(NodeStatus::Running),
            "SUCCESS" | "Success" => Ok(NodeStatus::Success),
            "FAILURE" | "Failure" => Ok(NodeStatus::Failure),
            "SKIPPED" | "Skipped" => Ok(NodeStatus::Skipped),
            _ => Err(ParseNodeStatusError::NoMatch),
        }
    }
}

impl FromString for NodeType {
    type Err = ParseNodeTypeError;

    fn from_string(value: impl AsRef<str>) -> Result<NodeType, Self::Err> {
        match value.as_ref() {
            "Action" => Ok(NodeType::Action),
            "Condition" => Ok(NodeType::Condition),
            "Control" => Ok(NodeType::Control),
            "Decorator" => Ok(NodeType::Decorator),
            "SubTree" => Ok(NodeType::SubTree),
            _ => Err(ParseNodeTypeError::NoMatch),
        }
    }
}

impl FromString for PortDirection {
    type Err = ParsePortDirectionError;

    fn from_string(value: impl AsRef<str>) -> Result<PortDirection, Self::Err> {
        match value.as_ref() {
            "Input" | "INPUT" => Ok(PortDirection::Input),
            "Output" | "OUTPUT" => Ok(PortDirection::Output),
            "InOut" | "INOUT" => Ok(PortDirection::InOut),
            _ => Err(ParsePortDirectionError::NoMatch),
        }
    }
}

/// Custom implementation of converting a type into a `String`. Going to/from
/// strings is handled in a custom way in `behaviortree_rs` to maintain
/// compatibility with the original BehaviorTree.CPP library.
pub trait BTToString {
    /// Convert the type to a string
    fn bt_to_string(&self) -> String;
}

impl BTToString for String {
    fn bt_to_string(&self) -> String {
        self.clone()
    }
}

impl_into_string!(
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    bool,
    NodeStatus,
    NodeType,
    PortDirection,
    &str
);

// ===========================
// End of String Conversions
// ===========================

#[derive(Debug, Default, Clone, PartialEq)]
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

pub trait DynPortValue: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_boxed(&self) -> Box<dyn DynPortValue>;
}

impl<T> DynPortValue for T
where
    T: Any + Clone + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn DynPortValue> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn DynPortValue> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}

impl std::fmt::Debug for Box<dyn DynPortValue> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Box {{ .. }}")
    }
}

pub struct PortInfoBuilder<T> {
    name: String,
    direction: PortDirection,
    description: Option<String>,
    default_value: Option<Box<dyn DynPortValue>>,
    parse_expr: bool,
    type_id: TypeId,
    _pd: PhantomData<T>,
}

/// Metadata about a node port
#[derive(Debug, Clone)]
pub struct PortInfo {
    name: String,
    /// Direction category for the port
    direction: PortDirection,
    type_id: TypeId,
    /// Optional description of the port
    description: String,
    default_value: Option<Box<dyn DynPortValue>>,
    /// When `true`, should parse the port value as an expression when loading
    /// the tree to validate syntax.
    parse_expr: bool,
}

impl<T> PortInfoBuilder<T>
where
    T: DynPortValue + 'static,
{
    pub fn build(self) -> PortInfo {
        PortInfo {
            name: self.name,
            direction: self.direction,
            type_id: self.type_id,
            description: self.description.unwrap_or_default(),
            default_value: self.default_value,
            parse_expr: self.parse_expr,
        }
    }

    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);

        self
    }

    pub fn default_value(mut self, value: impl Into<T>) -> Self {
        self.default_value = Some(Box::new(value.into()));

        self
    }

    pub fn parse_expr(mut self) -> Self {
        self.parse_expr = true;

        self
    }
}

impl PortInfo {
    pub fn input<T: DynPortValue + 'static>(name: impl Into<String>) -> PortInfoBuilder<T> {
        PortInfoBuilder {
            name: name.into(),
            direction: PortDirection::Input,
            type_id: TypeId::of::<T>(),
            description: None,
            default_value: None,
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
                .and_then(|val| val.as_any().downcast_ref())
        }
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description
    }

    pub fn set_expr(&mut self, parse_expr: bool) {
        self.parse_expr = parse_expr;
    }

    pub fn parse_expr(&self) -> bool {
        self.parse_expr
    }

    pub fn direction(&self) -> PortDirection {
        self.direction
    }
}

impl PartialEq for PortInfo {
    fn eq(&self, other: &Self) -> bool {
        // Does not check equality between default values
        self.name == other.name
            && self.direction == other.direction
            && self.type_id == other.type_id
            && self.description == other.description
            && self.parse_expr == other.parse_expr
    }
}

/// Remap a blackboard key
pub(crate) fn get_remapped_key(port_name: &str, remapped_port: &str) -> Option<String> {
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
