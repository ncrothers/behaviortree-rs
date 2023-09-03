use std::{
    fmt::Debug,
    str::FromStr, collections::HashMap, any::Any, ffi::IntoStringError, convert::Infallible,
};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

use crate::{macros::{impl_string_into, impl_into_string}, nodes::PortClone};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Undefined,
    Action,
    Condition,
    Control,
    Decorator,
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
        match self {
            Self::Idle | Self::Skipped => false,
            _ => true,
        }
    }

    pub fn is_completed(&self) -> bool {
        match self {
            Self::Success | Self::Failure => true,
            _ => false,
        }
    }

    pub fn into_string_color(&self) -> String {
        let color_start = match self {
            Self::Idle => "\x1b[36m",
            Self::Running => "\x1b[33m",
            Self::Success => "\x1b[32m",
            Self::Failure => "\x1b[31m",
            Self::Skipped => "\x1b[34m",
        };

        String::from(color_start.to_string() + &self.bt_to_string() + "\x1b[0m")
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

/// Trait for custom conversion into String
///
/// Out of the box, `StringInto<T>` is implemented on all numeric types, `bool`,
/// `NodeStatus`, `NodeType`, and `PortDirection`, and `Vec`s holding those types.
///
/// To implement `StringInto<T>` on your own type, it is recommended to implement `FromStr` on your type,
/// then call the `impl_string_into!` macro and pass your custom type in. Here's an example:
///
/// ```rust
/// struct MyType {
///     foo: String
/// }
///
/// impl std::str::FromStr for MyType {
///     type Err = ParseError;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         todo!()
///     }
/// }
///
/// impl_string_into!(MyType);
/// ```
pub trait StringInto<T> {
    type Err;

    fn string_into(&self) -> Result<T, Self::Err>;
}

impl_string_into!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

impl StringInto<String> for String {
    type Err = Infallible;

    fn string_into(&self) -> Result<String, Self::Err> {
        Ok(self.clone())
    }
}

impl<T> StringInto<Vec<String>> for T
where T: AsRef<str>
{
    type Err = Infallible;

    fn string_into(&self) -> Result<Vec<String>, Self::Err> {
        self
            .as_ref()
            .split(";")
            .map(|x| Ok(x.to_string()))
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum ParseBoolError {
    #[error("string wasn't one of the expected: 1/0, true/false, TRUE/FALSE")]
    ParseError,
}

impl<T> StringInto<bool> for T
where
    T: AsRef<str>,
{
    type Err = ParseBoolError;

    fn string_into(&self) -> Result<bool, Self::Err> {
        match self.as_ref() {
            "1" | "true" | "TRUE" => Ok(true),
            "0" | "false" | "FALSE" => Ok(false),
            _ => Err(ParseBoolError::ParseError),
        }
    }
}

impl<T> StringInto<NodeStatus> for T
where
    T: AsRef<str>,
{
    type Err = ParseNodeStatusError;

    fn string_into(&self) -> Result<NodeStatus, Self::Err> {
        match self.as_ref() {
            "IDLE" => Ok(NodeStatus::Idle),
            "RUNNING" => Ok(NodeStatus::Idle),
            "SUCCESS" => Ok(NodeStatus::Idle),
            "FAILURE" => Ok(NodeStatus::Idle),
            "SKIPPED" => Ok(NodeStatus::Idle),
            _ => Err(ParseNodeStatusError::NoMatch),
        }
    }
}

impl<T> StringInto<NodeType> for T
where
    T: AsRef<str>,
{
    type Err = ParseNodeTypeError;

    fn string_into(&self) -> Result<NodeType, Self::Err> {
        match self.as_ref() {
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

impl<T> StringInto<PortDirection> for T
where
    T: AsRef<str>,
{
    type Err = ParsePortDirectionError;

    fn string_into(&self) -> Result<PortDirection, Self::Err> {
        match self.as_ref() {
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
    serde_json::Value,
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

// pub trait PortInfoTrait {
//     fn set_description(&mut self, description: String) {
//         self.description = description
//     }

//     fn direction(&self) -> &PortDirection {
//         &self.r#type
//     }
// }

pub trait PortValue: Any + PortClone + Debug + BTToString {}

impl<T> PortClone for T
where
    T: 'static + Any + Clone + Debug + BTToString,
{
    fn clone_port(&self) -> Box<dyn PortValue> {
        Box::new(self.clone())
    }
}

impl<T> PortValue for T
where T: Any + PortClone + Debug + BTToString
{}

#[derive(Clone, Debug)]
pub struct PortInfo {
    r#type: PortDirection,
    description: String,
    default_value: Option<Box<dyn PortValue>>,
}

impl PortInfo {
    pub fn new(direction: PortDirection) -> PortInfo {
        Self {
            r#type: direction,
            description: String::new(),
            default_value: None,
        }
    }

    pub fn default_value(&self) -> Option<&Box<dyn PortValue>> {
        match &self.default_value {
            Some(v) => Some(&v),
            None => None,
        }
    }

    pub fn default_value_str(&self) -> Option<String> {
        match &self.default_value {
            Some(v) => Some(v.bt_to_string()),
            None => None,
        }
    }

    pub fn set_default(&mut self, default: impl PortValue) {
        self.default_value = Some(Box::new(default))
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

    pub fn default(mut self, default: impl PortValue) -> Port {
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