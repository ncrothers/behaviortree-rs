use std::string::FromUtf8Error;

use evalexpr::{DefaultNumericTypes, EvalexprError};
use thiserror::Error;
use typed_builder::TypedBuilder;

use crate::{
    basic_types::{NodeStatus, ParseBoolError},
    blackboard::Blackboard,
    node_registry::NodeRegistry,
    nodes::{NodeResult, TreeNode},
    Parser,
};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Port name [{0}] did not match Node [{1}] port list: {2:?}")]
    /// `(port_name, node_name, port_list)`
    InvalidPort(String, String, Vec<String>),
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
    #[error("Error parsing expression in port value: {0}")]
    InvalidPortExpression(#[from] EvalexprError<DefaultNumericTypes>),
    #[error("Variable in blackboard pointer \"{0}\" is missing a type.")]
    PortExpressionMissingType(String),
    #[error("Invalid type \"{type_name}\" for variable \"{ident}\". Valid types are: int, float, str, bool")]
    PortExpressionInvalidType { ident: String, type_name: String },
}

enum TickOption {
    WhileRunning,
    ExactlyOnce,
    OnceUnlessWokenUp,
}

pub struct NodeIter<'a> {
    nodes: Vec<&'a TreeNode>,
    idxs: Vec<i32>,
}

/// Configuration passed to [`Tree::from_config`] to build a [`Tree`] from XML
/// text.
///
/// ```
/// use behaviortree_rs::prelude::*;
///
/// let mut registry = NodeRegistry::default();
///
/// // Insert custom nodes into the registry here
///
/// let xml = r#"
/// <root>
///     <BehaviorTree ID="main-tree">
///         <Condition />
///     </BehaviorTree>
/// </root>
/// "#;
///
/// let config = TreeConfig::builder()
///     .registry(&registry)
///     .xml(xml)
///     // Optional
///     .tree_name("main-tree")
///     // Optional
///     .blackboard(Blackboard::default())
///     .build();
///
/// let tree = Tree::from_config(&config);
///
/// assert!(tree.is_ok());
/// ```
#[derive(TypedBuilder)]
pub struct TreeConfig<'a> {
    /// Holds all registered nodes
    pub(crate) registry: &'a NodeRegistry,
    /// XML text to parse the tree from
    pub(crate) xml: &'a str,
    /// Optional. Specify which tree to build by ID.
    ///
    /// When this is `None`, the `main_tree_to_execute` value will be used if set.
    /// If there is no `main_tree_to_execute` attribute set, there must be only
    /// one behavior tree defined in the XML text, otherwise the build will fail.
    #[builder(default, setter(strip_option))]
    pub(crate) tree_name: Option<&'a str>,
    /// Optional. Provide an external [`Blackboard`] to use as the root for this
    /// tree. If not provided, a new one will be allocated and used as the root.
    #[builder(default)]
    pub(crate) blackboard: Blackboard,
}

/// Top-level container of a behavior tree. Provides methods to tick the tree,
/// access the root-level [`Blackboard`], and iterate over its nodes.
///
/// ```
/// use behaviortree_rs::prelude::*;
///
/// let mut registry = NodeRegistry::default();
///
/// // Insert custom nodes into the registry here
///
/// let xml = r#"
/// <root>
///     <BehaviorTree ID="main-tree">
///         <Condition />
///     </BehaviorTree>
/// </root>
/// "#;
///
/// let config = TreeConfig::builder()
///     .registry(&registry)
///     .xml(xml)
///     // Optional
///     .tree_name("main-tree")
///     // Optional
///     .blackboard(Blackboard::default())
///     .build();
///
/// let tree = Tree::from_config(&config);
///
/// assert!(tree.is_ok());
/// ```
#[derive(Debug)]
pub struct Tree {
    root: TreeNode,
}

impl Tree {
    pub(crate) fn new(root: TreeNode) -> Tree {
        Self { root }
    }

    /// Creates a behavior tree from the [`TreeConfig`].
    pub fn from_config(config: &TreeConfig) -> Result<Self, ParseError> {
        let mut parser = Parser::new(config);

        parser.create_tree()
    }

    fn tick_root(&mut self, opt: TickOption) -> NodeResult {
        let mut status = NodeStatus::Idle;

        while status == NodeStatus::Idle
            || (matches!(opt, TickOption::WhileRunning) && matches!(status, NodeStatus::Running))
        {
            status = self.root.execute_tick()?;

            // Not implemented: Check for wake-up conditions and tick again if so

            if status.is_completed() {
                self.root.reset_status();
            }
        }

        Ok(status)
    }

    pub fn tick_exactly_once(&mut self) -> NodeResult {
        self.tick_root(TickOption::ExactlyOnce)
    }

    pub fn tick_once(&mut self) -> NodeResult {
        self.tick_root(TickOption::OnceUnlessWokenUp)
    }

    pub fn tick_while_running(&mut self) -> NodeResult {
        self.tick_root(TickOption::WhileRunning)
    }

    pub fn root_blackboard(&self) -> Blackboard {
        self.root.data.blackboard().clone()
    }

    pub fn halt_tree(&mut self) -> NodeResult<()> {
        self.root.halt()
    }

    pub fn visit_nodes(&self) -> impl Iterator<Item = &TreeNode> {
        NodeIter::new(&self.root)
    }
}

impl<'a> NodeIter<'a> {
    pub fn new(root: &'a TreeNode) -> Self {
        Self {
            nodes: vec![root],
            idxs: vec![-1],
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a TreeNode;

    fn next(&mut self) -> Option<Self::Item> {
        // Loop until we find a node to return
        loop {
            // Out of nodes; we have traversed the entire tree
            if self.nodes.is_empty() {
                return None;
            }

            let end_idx = self.nodes.len() - 1;

            let node = self.nodes[end_idx];
            let child_idx = &mut self.idxs[end_idx];

            // When this index is -1, that means we haven't returned the node yet
            if *child_idx < 0 {
                self.idxs[end_idx] = 0;
                return Some(node);
            } else if node.children().is_none()
                || *child_idx >= node.children().unwrap().len() as i32
            {
                // When the node has no children, pop it off and try the next element
                // OR
                // If we've already returned all children, pop it off
                // Unwrap is safe because we just checked if it's None
                self.nodes.pop();
                self.idxs.pop();
            } else {
                // If nothing else applies, we can push the node's child and return it
                // Unwrap is safe because we just checked if it's None
                let child = &node.children().unwrap()[*child_idx as usize];
                *child_idx += 1;

                self.nodes.push(child);
                self.idxs.push(-1);
            }
        }
    }
}

#[cfg(test)]
mod tests {}
