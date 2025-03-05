use std::{str::ParseBoolError, string::FromUtf8Error};

#[cfg(feature = "expr")]
use evalexpr::{DefaultNumericTypes, EvalexprError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    /// `(port_name, node_name, port_list)`
    #[error("Port name [{0}] did not match Node [{1}] port list: {2:?}")]
    InvalidPort(String, String, Vec<String>),
    /// `(port_name, node_name, port_list)`
    #[error("Node [{node}] missing required port [{port}]")]
    MissingRequiredPort { node: String, port: String },
    #[error("Error occurred parsing XML attribute: {0}")]
    AttrError(#[from] quick_xml::events::attributes::AttrError),
    #[error("Error occurred parsing XML: {0}")]
    XMLError(#[from] quick_xml::Error),
    #[error("Expected to find <root> start tag at start of XML. Found incorrect tag.")]
    MissingRoot,
    #[error("Expected to find <root> tag at start of XML. Found <{0}> instead.")]
    ExpectedRoot(String),
    #[error("Reached EOF of the XML unexpectedly.")]
    UnexpectedEof,
    #[error("Error parsing UTF8: {0}")]
    Utf8Error(#[from] FromUtf8Error),
    #[error("Attempted to parse node with unregistered name: {0}")]
    UnknownNode(String),
    #[error("Errors like this shouldn't happen. {0}")]
    InternalError(String),
    #[error("{0}")]
    MissingAttribute(String),
    #[error("Can't find tree [{0}]")]
    UnknownTree(String),
    #[error("Node type [] didn't had invalid presence/absence of children.")]
    NodeTypeMismatch(String),
    #[error("No main tree was provided, either in the XML or as a function parameter.")]
    NoMainTree,
    #[error("{0}")]
    ParseStringError(#[from] ParseBoolError),
    #[error("Violated node type constraint: {0}")]
    ViolateNodeConstraint(String),
    #[cfg(feature = "expr")]
    #[error("Error parsing expression in port value: {0}")]
    InvalidPortExpression(#[from] EvalexprError<DefaultNumericTypes>),
    #[error("Variable in blackboard pointer \"{0}\" is missing a type.")]
    PortExpressionMissingType(String),
    #[error("Invalid type \"{type_name}\" for variable \"{ident}\". Valid types are: int, float, str, bool")]
    PortExpressionInvalidType { ident: String, type_name: String },
}
