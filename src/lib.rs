use std::{str::FromStr, fmt::Debug, num::{ParseIntError, ParseFloatError}};

use num_traits::PrimInt;
use thiserror::Error;

pub enum NodeType {
    Undefined,
    Action,
    Condition,
    Control,
    Decorator,
    SubTree,
}

pub enum NodeStatus {
    Idle,
    Running,
    Success,
    Failure,
    Skipped,
}

#[derive(Error, Debug)]
pub enum ParseNodeStatusError {
    #[error("string didn't match any NodeStatus values")]
    NoMatch
}

#[derive(Error, Debug)]
pub enum ParseNodeTypeError {
    #[error("string didn't match any NodeType values")]
    NoMatch
}

#[derive(Error, Debug)]
pub enum ParsePortDirectionError {
    #[error("string didn't match any PortDirection values")]
    NoMatch
}

impl NodeStatus {
    pub fn is_active(&self) -> bool {
        match self {
            Self::Idle | Self::Skipped => false,
            _ => true
        }
    }

    pub fn is_completed(&self) -> bool {
        match self {
            Self::Success | Self::Failure => true,
            _ => false
        }
    }
}

pub enum PortDirection {
    Input,
    Output,
    InOut,
}

// ===========================
// Converting string to types
// ===========================

/// Trait for custom conversion 
pub trait StringInto<T> {
    type Err;

    fn string_into(&self) -> Result<T, Self::Err>;
}

#[derive(Error, Debug)]
pub enum ParseBoolError {
    #[error("string wasn't one of the expected: 1/0, true/false, TRUE/FALSE")]
    ParseError,
}

impl<T> StringInto<bool> for T
where T: AsRef<str>
{
    type Err = ParseBoolError;

    fn string_into(&self) -> Result<bool, Self::Err> {
        match self.as_ref() {
            "1" | "true" | "TRUE" => Ok(true),
            "0" | "false" | "FALSE" => Ok(false),
            _ => Err(ParseBoolError::ParseError)
        }
    }
}

impl<T> StringInto<u8> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<u8, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<u16> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<u16, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<u32> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<u32, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<u64> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<u64, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<u128> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<u128, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<usize> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<usize, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<i8> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<i8, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<i16> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<i16, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<i32> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<i32, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<i64> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<i64, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<i128> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<i128, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<isize> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<isize, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<f32> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseFloatError;

    fn string_into(&self) -> Result<f32, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<f64> for T
where T: AsRef<str> + FromStr
{
    type Err = ParseFloatError;

    fn string_into(&self) -> Result<f64, Self::Err> {
        self.as_ref().parse()
    }
}

impl<T> StringInto<Vec<f32>> for T
where T: AsRef<str>
{
    type Err = ParseFloatError;

    fn string_into(&self) -> Result<Vec<f32>, Self::Err> {
        self
            .as_ref()
            .split(";")
            .map(|x| Ok(x.parse()?))
            .collect()
    }
}

impl<T> StringInto<Vec<i32>> for T
where T: AsRef<str>
{
    type Err = ParseIntError;

    fn string_into(&self) -> Result<Vec<i32>, Self::Err> {
        self
            .as_ref()
            .split(";")
            .map(|x| Ok(x.parse()?))
            .collect()
    }
}

/// Trait with `try_to_vec()` method, which converts a type to a `Vec<T>`
pub trait TryToVec<T> {
    type Error;

    fn try_to_vec(&self) -> Result<Vec<T>, Self::Error>;
}

impl<T> TryToVec<T> for String
where T: FromStr
{
    type Error = T::Err;

    fn try_to_vec(&self) -> Result<Vec<T>, Self::Error> {
        self
            .split(";")
            .map(|x| Ok(x.parse()?))
            .collect()
    }
}

impl<T> TryToVec<T> for &str
where T: FromStr
{
    type Error = T::Err;

    fn try_to_vec(&self) -> Result<Vec<T>, Self::Error> {
        self
            .split(";")
            .map(|x| Ok(x.parse()?))
            .collect()
    }
}

impl FromStr for NodeStatus {
    type Err = ParseNodeStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "IDLE" => Ok(NodeStatus::Idle),
            "RUNNING" => Ok(NodeStatus::Idle),
            "SUCCESS" => Ok(NodeStatus::Idle),
            "FAILURE" => Ok(NodeStatus::Idle),
            "SKIPPED" => Ok(NodeStatus::Idle),
            _ => Err(ParseNodeStatusError::NoMatch)
        }
    }
}

impl FromStr for NodeType {
    type Err = ParseNodeTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNDEFINED" => Ok(NodeType::Undefined),
            "ACTION" => Ok(NodeType::Action),
            "CONDITION" => Ok(NodeType::Condition),
            "CONTROL" => Ok(NodeType::Control),
            "DECORATOR" => Ok(NodeType::Decorator),
            "SUBTREE" => Ok(NodeType::SubTree),
            _ => Err(ParseNodeTypeError::NoMatch)
        }
    }
}

impl FromStr for PortDirection {
    type Err = ParsePortDirectionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "INPUT" => Ok(PortDirection::Input),
            "OUTPUT" => Ok(PortDirection::Output),
            "INOUT" => Ok(PortDirection::InOut),
            _ => Err(ParsePortDirectionError::NoMatch)
        }
    }
}

pub struct PortsList;

pub struct TreeNodeManifest {
    node_type: NodeType,
    registration_id: String,
    ports: PortsList,
    description: String,
}

