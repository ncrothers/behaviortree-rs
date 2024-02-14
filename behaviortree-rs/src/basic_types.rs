use std::{any::Any, collections::HashMap, convert::Infallible, fmt::Debug, str::FromStr};

use quick_xml::events::attributes::Attributes;
use thiserror::Error;

use crate::{
    blackboard::BlackboardString,
    macros::{impl_from_string, impl_into_string},
    tree::ParseError,
};

/// Specifies all types of nodes that can be used in a behavior tree.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Undefined,
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
            Self::Undefined => "Undefined",
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
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
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

#[derive(Error, Debug)]
pub enum ParseBoolError {
    #[error("string wasn't one of the expected: 1/0, true/false, TRUE/FALSE")]
    ParseError,
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
            "Undefined" => Ok(NodeType::Undefined),
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

pub trait BTToString {
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

pub type PortsList = HashMap<String, PortInfo>;

#[derive(Clone, Debug)]
pub struct TreeNodeManifest {
    pub node_type: NodeType,
    pub registration_id: String,
    pub ports: PortsList,
    pub description: String,
}

impl TreeNodeManifest {
    pub fn new(
        node_type: NodeType,
        registration_id: impl AsRef<str>,
        ports: PortsList,
        description: impl AsRef<str>,
    ) -> TreeNodeManifest {
        Self {
            node_type,
            registration_id: registration_id.as_ref().to_string(),
            ports,
            description: description.as_ref().to_string(),
        }
    }
}

// ===========================
// Ports
// ===========================

pub trait PortChecks {
    fn is_allowed_port_name(&self) -> bool;
}

impl<T: AsRef<str>> PortChecks for T {
    fn is_allowed_port_name(&self) -> bool {
        let name = self.as_ref();

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
}

pub type PortsRemapping = HashMap<String, String>;

pub trait PortClone {
    fn clone_port(&self) -> Box<dyn PortValue>;
}

pub trait PortValue: Any + PortClone + Debug + BTToString {}

impl<T> PortClone for T
where
    T: 'static + Any + Clone + Debug + BTToString,
{
    fn clone_port(&self) -> Box<dyn PortValue> {
        Box::new(self.clone())
    }
}

impl<T> PortValue for T where T: Any + PortClone + Debug + BTToString {}

#[derive(Clone, Debug)]
pub struct PortInfo {
    r#type: PortDirection,
    description: String,
    default_value: Option<String>,
}

impl PortInfo {
    pub fn new(direction: PortDirection) -> PortInfo {
        Self {
            r#type: direction,
            description: String::new(),
            default_value: None,
        }
    }

    pub fn default_value(&self) -> Option<&String> {
        match &self.default_value {
            Some(v) => Some(v),
            None => None,
        }
    }

    pub fn default_value_str(&self) -> Option<String> {
        self.default_value.as_ref().map(|v| v.bt_to_string())
    }

    pub fn set_default(&mut self, default: impl BTToString) {
        self.default_value = Some(default.bt_to_string())
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description
    }

    pub fn direction(&self) -> &PortDirection {
        &self.r#type
    }
}

pub struct Port(String, PortInfo);

impl Port {
    fn create_port(direction: PortDirection, name: &str, description: &str) -> Port {
        let mut port_info = PortInfo::new(direction);
        port_info.set_description(description.to_string());

        Port(name.to_string(), port_info)
    }

    pub fn default(mut self, default: impl BTToString) -> Port {
        self.1.set_default(default);
        self
    }

    pub fn input(name: &str) -> Port {
        Self::input_description(name, "")
    }

    pub fn input_description(name: &str, description: &str) -> Port {
        Self::create_port(PortDirection::Input, name, description)
    }

    pub fn output(name: &str) -> Port {
        Self::output_description(name, "")
    }

    pub fn output_description(name: &str, description: &str) -> Port {
        Self::create_port(PortDirection::Output, name, description)
    }
}

pub fn get_remapped_key(
    port_name: impl AsRef<str>,
    remapped_port: impl AsRef<str>,
) -> Option<String> {
    if port_name.as_ref() == "=" {
        Some(port_name.as_ref().to_string())
    } else {
        remapped_port.as_ref().strip_bb_pointer()
    }
}

// ===========================
// Private Helpers
// ===========================

pub trait AttrsToMap {
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
