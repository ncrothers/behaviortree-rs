use core::str;
use std::{collections::HashMap, io::Cursor, str::Utf8Error, string::FromUtf8Error, sync::Arc};

use evalexpr::{DefaultNumericTypes, EvalexprError};
use log::{debug, info};
use quick_xml::{
    events::{attributes::Attributes, BytesStart, Event},
    Reader,
};
use thiserror::Error;

use crate::{
    basic_types::{
        AttrsToMap, FromString, NodeCategory, NodeStatus, ParseBoolError, PortChecks, PortDirection,
    },
    blackboard::{Blackboard, BlackboardString},
    macros::build_node_ptr,
    nodes::{self, NodeConfig, NodeResult, TreeNode},
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
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("Error parsing UTF8: {0}")]
    Utf8Error(#[from] Utf8Error),
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

type NodeCreateFnDyn = dyn Fn(NodeConfig, Vec<TreeNode>) -> TreeNode + Send + Sync;

enum TickOption {
    WhileRunning,
    ExactlyOnce,
    OnceUnlessWokenUp,
}

pub struct NodeIter<'a> {
    nodes: Vec<&'a TreeNode>,
    idxs: Vec<i32>,
}

#[derive(Debug)]
enum CreateNodeResult {
    Node(TreeNode),
    Continue,
    End,
}

#[derive(Debug)]
pub struct AsyncTree {
    root: TreeNode,
}

impl AsyncTree {
    pub fn new(root: TreeNode) -> AsyncTree {
        Self { root }
    }

    async fn tick_root(&mut self, opt: TickOption) -> NodeResult {
        let mut status = NodeStatus::Idle;

        while status == NodeStatus::Idle
            || (matches!(opt, TickOption::WhileRunning) && matches!(status, NodeStatus::Running))
        {
            status = self.root.execute_tick().await?;

            // Not implemented: Check for wake-up conditions and tick again if so

            if status.is_completed() {
                self.root.reset_status();
            }
        }

        Ok(status)
    }

    pub async fn tick_exactly_once(&mut self) -> NodeResult {
        self.tick_root(TickOption::ExactlyOnce).await
    }

    pub async fn tick_once(&mut self) -> NodeResult {
        self.tick_root(TickOption::OnceUnlessWokenUp).await
    }

    pub async fn tick_while_running(&mut self) -> NodeResult {
        self.tick_root(TickOption::WhileRunning).await
    }

    pub async fn root_blackboard(&self) -> Blackboard {
        self.root.config().blackboard.clone()
    }

    pub async fn halt_tree(&mut self) {
        self.root.halt().await;
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

#[derive(Debug)]
pub struct SyncTree {
    root: AsyncTree,
}

impl SyncTree {
    pub fn new(root: TreeNode) -> SyncTree {
        Self {
            root: AsyncTree::new(root),
        }
    }

    pub fn tick_exactly_once(&mut self) -> NodeResult {
        futures::executor::block_on(self.root.tick_exactly_once())
    }

    pub fn tick_once(&mut self) -> NodeResult {
        futures::executor::block_on(self.root.tick_once())
    }

    pub fn tick_while_running(&mut self) -> NodeResult {
        futures::executor::block_on(self.root.tick_while_running())
    }

    pub fn root_blackboard(&self) -> Blackboard {
        futures::executor::block_on(self.root.root_blackboard())
    }

    pub async fn halt_tree(&mut self) {
        futures::executor::block_on(self.root.halt_tree());
    }

    pub fn visit_nodes(&self) -> impl Iterator<Item = &TreeNode> {
        NodeIter::new(&self.root.root)
    }
}

pub struct Factory {
    node_map: HashMap<String, (NodeCategory, Arc<NodeCreateFnDyn>)>,
    blackboard: Blackboard,
    tree_roots: HashMap<String, Reader<Cursor<Vec<u8>>>>,
    main_tree_id: Option<String>,
    // TODO: temporary solution, potentially replace later
    tree_uid: std::sync::Mutex<u32>,
}

impl Factory {
    pub fn new() -> Factory {
        let blackboard = Blackboard::create();

        Self {
            node_map: builtin_nodes(),
            blackboard,
            tree_roots: HashMap::new(),
            main_tree_id: None,
            tree_uid: std::sync::Mutex::new(0),
        }
    }

    pub fn blackboard(&mut self) -> &Blackboard {
        &self.blackboard
    }

    pub fn set_blackboard(&mut self, blackboard: Blackboard) {
        self.blackboard = blackboard;
    }

    pub fn register_node<F>(&mut self, name: impl AsRef<str>, node_fn: F, node_type: NodeCategory)
    where
        F: Fn(NodeConfig, Vec<TreeNode>) -> TreeNode + Send + Sync + 'static,
    {
        self.node_map
            .insert(name.as_ref().into(), (node_type, Arc::new(node_fn)));
    }

    fn create_node(
        &self,
        node_fn: &Arc<NodeCreateFnDyn>,
        config: NodeConfig,
        children: Vec<TreeNode>,
    ) -> TreeNode {
        node_fn(config, children)
    }

    fn get_uid(&self) -> u32 {
        let uid = *self.tree_uid.lock().unwrap();
        *self.tree_uid.lock().unwrap() += 1;

        uid
    }

    fn recursively_build_subtree(
        &self,
        tree_id: &str,
        tree_name: &str,
        path_prefix: &str,
        blackboard: Blackboard,
    ) -> Result<TreeNode, ParseError> {
        let mut reader = match self.tree_roots.get(tree_id) {
            Some(root) => root.clone(),
            None => {
                return Err(ParseError::UnknownTree(tree_id.to_owned()));
            }
        };

        // Loop until either a child or end tag is found
        loop {
            match self.build_child(&mut reader, &blackboard, tree_name, path_prefix)? {
                CreateNodeResult::Node(child) => break Ok(child),
                CreateNodeResult::Continue => (),
                CreateNodeResult::End => {
                    break Err(ParseError::NodeTypeMismatch("SubTree".to_string()))
                }
            }
        }
    }

    pub fn create_sync_tree_from_text(
        &mut self,
        text: String,
        blackboard: &Blackboard,
    ) -> Result<SyncTree, ParseError> {
        self.register_bt_from_text(text)?;

        if self.tree_roots.len() > 1 && self.main_tree_id.is_none() {
            Err(ParseError::NoMainTree)
        } else if self.tree_roots.len() == 1 {
            // Unwrap is safe because we check that tree_roots.len() == 1
            let main_tree_id = self.tree_roots.iter().next().unwrap().0.clone();

            self.instantiate_sync_tree(blackboard, &main_tree_id)
        } else {
            // Unwrap is safe here because there are more than 1 root and
            // self.main_tree_id is Some
            let main_tree_id = self.main_tree_id.clone().unwrap();
            self.instantiate_sync_tree(blackboard, &main_tree_id)
        }
    }

    pub fn create_async_tree_from_text(
        &mut self,
        text: String,
        blackboard: &Blackboard,
    ) -> Result<AsyncTree, ParseError> {
        self.register_bt_from_text(text)?;

        if self.tree_roots.len() > 1 && self.main_tree_id.is_none() {
            Err(ParseError::NoMainTree)
        } else if self.tree_roots.len() == 1 {
            // Unwrap is safe because we check that tree_roots.len() == 1
            let main_tree_id = self.tree_roots.iter().next().unwrap().0.clone();

            self.instantiate_async_tree(blackboard, &main_tree_id)
        } else {
            // Unwrap is safe here because there are more than 1 root and
            // self.main_tree_id is Some
            let main_tree_id = self.main_tree_id.clone().unwrap();
            self.instantiate_async_tree(blackboard, &main_tree_id)
        }
    }

    pub fn instantiate_sync_tree(
        &mut self,
        blackboard: &Blackboard,
        main_tree_id: &str,
    ) -> Result<SyncTree, ParseError> {
        // Clone ptr to Blackboard
        let blackboard = blackboard.clone();

        let main_tree_id = String::from(main_tree_id);

        let root_node = self.recursively_build_subtree(&main_tree_id, "", "", blackboard)?;

        Ok(SyncTree::new(root_node))
    }

    pub fn instantiate_async_tree(
        &mut self,
        blackboard: &Blackboard,
        main_tree_id: &str,
    ) -> Result<AsyncTree, ParseError> {
        // Clone ptr to Blackboard
        let blackboard = blackboard.clone();

        let main_tree_id = String::from(main_tree_id);

        let root_node = self.recursively_build_subtree(&main_tree_id, "", "", blackboard)?;

        Ok(AsyncTree::new(root_node))
    }

    fn build_leaf_node(
        &self,
        node_name: &str,
        attributes: Attributes,
        config: NodeConfig,
    ) -> Result<TreeNode, ParseError> {
        // Get clone of node from node_map based on tag name
        let (node_type, node_fn) = self
            .node_map
            .get(node_name)
            .ok_or_else(|| ParseError::UnknownNode(node_name.to_owned()))?;
        if !matches!(node_type, NodeCategory::Action) {
            return Err(ParseError::NodeTypeMismatch(String::from("Action")));
        }

        let mut node = self.create_node(node_fn, config, Vec::new());

        self.add_ports_to_node(&mut node, node_name, attributes)?;

        Ok(node)
    }

    fn build_children(
        &self,
        reader: &mut Reader<Cursor<Vec<u8>>>,
        blackboard: &Blackboard,
        tree_name: &str,
        path_prefix: &str,
    ) -> Result<Vec<TreeNode>, ParseError> {
        let mut nodes = Vec::new();

        loop {
            match self.build_child(reader, blackboard, tree_name, path_prefix)? {
                CreateNodeResult::Node(node) => {
                    nodes.push(node);
                }
                CreateNodeResult::Continue => (),
                CreateNodeResult::End => break,
            }
        }

        Ok(nodes)
    }

    fn add_ports_to_node(
        &self,
        node_ptr: &mut TreeNode,
        node_name: &str,
        attributes: Attributes,
    ) -> Result<(), ParseError> {
        let config = node_ptr.config_mut();
        let manifest = config.manifest()?;

        let attrs = attributes.to_map()?;

        // Check if all ports from XML match ports in manifest
        for port_name in attrs.keys() {
            if !manifest.ports.contains_key(port_name) {
                return Err(ParseError::InvalidPort(
                    port_name.clone(),
                    node_name.to_owned(),
                    manifest.ports.to_owned().into_keys().collect(),
                ));
            }
        }

        // Add ports to NodeConfig
        for (remap_name, remap_val) in attrs {
            if let Some(port) = manifest.ports.get(&remap_name) {
                // Validate that any expr-enabled ports contain valid expressions,
                // and the provided types for blackboard pointers are one of the valid ones
                if port.parse_expr() {
                    let expr =
                        evalexpr::build_operator_tree::<evalexpr::DefaultNumericTypes>(&remap_val)?;

                    for key in expr.iter_variable_identifiers() {
                        // Check if it's a blackboard pointer
                        if key.starts_with('{') && key.ends_with('}') {
                            // Remove the brackets
                            let inner_key = &key[1..(key.len() - 1)];
                            // Split the type from the name
                            let (name, var_type) = inner_key.split_once(':').ok_or_else(|| {
                                ::behaviortree_rs::tree::ParseError::PortExpressionMissingType(
                                    inner_key.to_owned(),
                                )
                            })?;

                            // Check if the type is supported
                            match var_type {
                                "int" | "float" | "str" | "bool" => (),
                                _ => return Err(::behaviortree_rs::tree::ParseError::PortExpressionInvalidType { ident: name.to_owned(), type_name: var_type.to_owned() }),
                            };
                        }
                    }
                }

                config.add_port(port.direction().clone(), remap_name, remap_val);
            }
        }

        // Try to use defaults for unspecified port values
        for (port_name, port_info) in manifest.ports.iter() {
            let direction = port_info.direction();

            if !matches!(direction, PortDirection::Output)
                && !config.has_port(direction, port_name)
                && port_info.default_value().is_some()
            {
                config.add_port(
                    PortDirection::Input,
                    port_name.clone(),
                    port_info.default_value().unwrap().to_owned(),
                );
            }
        }

        Ok(())
    }

    /// Builds a node that begins with a Start tag, which is one with children (Control or Decorator)
    fn build_start_tag(
        &self,
        e: BytesStart,
        reader: &mut Reader<Cursor<Vec<u8>>>,
        blackboard: &Blackboard,
        tree_name: &str,
        path_prefix: &str,
    ) -> Result<TreeNode, ParseError> {
        let node_name = str::from_utf8(e.name().0)?;
        let attributes = e.attributes();

        debug!("build_child Start: {node_name}");

        let mut config = NodeConfig::new(blackboard.clone());
        config.path = path_prefix.to_owned() + node_name;

        let (node_type, node_fn) = self
            .node_map
            .get(node_name)
            .ok_or_else(|| ParseError::UnknownNode(node_name.to_owned()))?;

        let children = match node_type {
            NodeCategory::Control => self.build_children(
                reader,
                blackboard,
                tree_name,
                &(config.path.to_owned() + "/"),
            )?,
            NodeCategory::Decorator => {
                // Loop until either an end tag or the child is found
                let child = loop {
                    match self.build_child(
                        reader,
                        blackboard,
                        tree_name,
                        &(config.path.to_owned() + "/"),
                    )? {
                        CreateNodeResult::Node(node) => break node,
                        CreateNodeResult::Continue => (),
                        CreateNodeResult::End => {
                            return Err(ParseError::NodeTypeMismatch("Decorator".to_string()))
                        }
                    }
                };

                let mut buf = Vec::new();

                // Try to match the end tag to close the Decorator
                loop {
                    match reader.read_event_into(&mut buf)? {
                        // Ignore comments
                        Event::Comment(_) => continue,
                        Event::End(tag) => {
                            // If a matching end tag is found, all good
                            if tag.name() == e.name() {
                                break;
                            } else {
                                // Otherwise, an error. Theoretically this should be unreachable since the XML parser should catch this error, but keeping it here just in case
                                return Err(ParseError::ViolateNodeConstraint(format!(
                                    "Expected end tag for Decorator {node_name}"
                                )));
                            }
                        }
                        _ => {
                            return Err(ParseError::ViolateNodeConstraint(format!(
                                "Decorator node [{node_name}] may only have one child"
                            )));
                        }
                    }
                }

                vec![child]
            }
            // TODO: expand more
            x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}"))),
        };

        let mut node = self.create_node(node_fn, config, children);

        self.add_ports_to_node(&mut node, node_name, attributes)?;

        Ok(node)
    }

    /// Builds a node that is an Empty tag, which is an Action or SubTree node of some kind.
    fn build_empty_tag(
        &self,
        e: BytesStart,
        blackboard: &Blackboard,
        tree_name: &str,
        path_prefix: &str,
    ) -> Result<TreeNode, ParseError> {
        let node_name = str::from_utf8(e.name().0)?;
        debug!("[Leaf node]: {node_name}");
        let attributes = e.attributes();

        let mut config = NodeConfig::new(blackboard.clone());
        config.path = path_prefix.to_owned() + node_name;

        let node = match node_name {
            "SubTree" => {
                let attributes = attributes.to_map()?;
                let mut child_blackboard = Blackboard::with_parent(blackboard);

                // Process attributes (Ports, special fields, etc)
                for (attr, value) in attributes.iter() {
                    // Set autoremapping to true or false
                    if attr == "_autoremap" {
                        child_blackboard
                            .enable_auto_remapping(<bool as FromString>::from_string(value)?);
                        continue;
                    } else if !attr.is_allowed_port_name() {
                        continue;
                    }

                    if let Some(port_name) = value.strip_bb_pointer() {
                        // Add remapping if `value` is a Blackboard pointer
                        child_blackboard.add_subtree_remapping(attr.clone(), port_name.to_owned());
                    } else {
                        // Set string value into Blackboard
                        child_blackboard.set(attr, value.clone());
                    }
                }

                let id = match attributes.get("ID") {
                    Some(id) => id,
                    None => return Err(ParseError::MissingAttribute("ID".to_string())),
                };

                let mut subtree_name = tree_name.to_owned();
                if !subtree_name.is_empty() {
                    subtree_name += "/";
                }

                if let Some(name_attr) = attributes.get("name") {
                    subtree_name += name_attr;
                } else {
                    subtree_name += &format!("{id}::{}", self.get_uid());
                }

                let new_prefix = format!("{subtree_name}/");

                self.recursively_build_subtree(id, &subtree_name, &new_prefix, child_blackboard)?
            }
            _ => self.build_leaf_node(node_name, attributes, config)?,
        };

        Ok(node)
    }

    /// Recursively build the child node
    fn build_child(
        &self,
        reader: &mut Reader<Cursor<Vec<u8>>>,
        blackboard: &Blackboard,
        tree_name: &str,
        path_prefix: &str,
    ) -> Result<CreateNodeResult, ParseError> {
        let mut buf = Vec::new();

        let node = match reader.read_event_into(&mut buf)? {
            // exits the loop when reaching end of file
            Event::Eof => {
                debug!("EOF");
                return Err(ParseError::UnexpectedEof);
            }
            // Node with Children
            Event::Start(e) => {
                self.build_start_tag(e, reader, blackboard, tree_name, path_prefix)?
            }
            // Leaf Node
            Event::Empty(e) => self.build_empty_tag(e, blackboard, tree_name, path_prefix)?,
            Event::End(_) => return Ok(CreateNodeResult::End),
            Event::Comment(content) => {
                debug!("Comment - \"{content:?}\"");
                return Ok(CreateNodeResult::Continue);
            }
            e => {
                debug!("Other - SHOULDN'T BE HERE");
                debug!("{e:?}");

                return Err(ParseError::InternalError(
                    "Didn't match one of the expected XML tag types.".to_string(),
                ));
            }
        };

        Ok(CreateNodeResult::Node(node))
    }

    /// Registers all of the BehaviorTrees defined in the XML string
    pub fn register_bt_from_text(&mut self, xml: String) -> Result<(), ParseError> {
        let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes().to_vec()));
        reader.trim_text(true);

        let mut buf = Vec::new();

        // TODO: Check includes

        // TODO: Parse for correctness

        loop {
            // Try to match root tag
            match reader.read_event_into(&mut buf)? {
                // Ignore XML declaration tag <?xml ...
                Event::Decl(_) => buf.clear(),
                Event::Start(e) => {
                    let name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes().to_map()?;

                    if name.as_str() != "root" {
                        buf.clear();
                        continue;
                    }

                    if let Some(tree_id) = attributes.get("main_tree_to_execute") {
                        info!("Found main tree ID: {tree_id}");
                        self.main_tree_id = Some(tree_id.clone());
                    }

                    buf.clear();
                    break;
                }
                _ => return Err(ParseError::MissingRoot),
            }
        }

        // Register each BehaviorTree in the XML
        loop {
            let event = { reader.read_event_into(&mut buf)? };

            match event {
                Event::Start(e) => {
                    let name = str::from_utf8(e.name().0)?;
                    let attributes = e.attributes().to_map()?;

                    let mut buf = Vec::new();

                    // TODO: Maybe do something with TreeNodesModel?
                    // For now, just ignore it
                    if name == "TreeNodesModel" {
                        reader.read_to_end_into(e.to_end().name(), &mut buf)?;
                    } else {
                        // Add error for missing BT
                        if name != "BehaviorTree" {
                            return Err(ParseError::ExpectedRoot(name.to_owned()));
                        }

                        // Save position of Reader for each BT
                        if let Some(id) = attributes.get("ID") {
                            self.tree_roots.insert(id.clone(), reader.clone());
                        } else {
                            return Err(ParseError::MissingAttribute("Found BehaviorTree definition without ID. Cannot continue parsing.".to_string()));
                        }

                        // Try to match the first node and skip past it
                        loop {
                            match reader.read_event_into(&mut buf)? {
                                // Ignore comments
                                Event::Comment(_) => continue,
                                Event::End(tag) => {
                                    // If a matching end tag is found, all good
                                    if tag.name() == e.name() {
                                        break;
                                    } else {
                                        // Otherwise, an error. Theoretically this should be unreachable since the XML parser should catch this error, but keeping it here just in case
                                        return Err(ParseError::ViolateNodeConstraint(
                                            String::from("Expected end tag for BehaviorTree"),
                                        ));
                                    }
                                }
                                Event::Start(e) => {
                                    let mut buf = Vec::new();
                                    reader.read_to_end_into(e.name(), &mut buf)?;
                                    break;
                                }
                                Event::Empty(_) => break,
                                _ => continue,
                            }
                        }

                        // Try to match the end tag to close the BehaviorTree
                        loop {
                            match reader.read_event_into(&mut buf)? {
                                // Ignore comments
                                Event::Comment(_) => continue,
                                Event::End(tag) => {
                                    // If a matching end tag is found, all good
                                    if tag.name() == e.name() {
                                        break;
                                    } else {
                                        // Otherwise, an error. Theoretically this should be unreachable since the XML parser should catch this error, but keeping it here just in case
                                        return Err(ParseError::ViolateNodeConstraint(
                                            String::from("Expected end tag for BehaviorTree"),
                                        ));
                                    }
                                }
                                _ => {
                                    return Err(ParseError::ViolateNodeConstraint(String::from(
                                        "BehaviorTree node may only have one child",
                                    )));
                                }
                            }
                        }
                    }
                }
                Event::End(e) => {
                    let name = String::from_utf8(e.name().0.into())?;
                    if name != "root" {
                        return Err(ParseError::InternalError("A non-root end tag was found. This should not happen. Please report this.".to_string()));
                    } else {
                        break;
                    }
                }
                Event::Comment(_) => (),
                x => {
                    return Err(ParseError::InternalError(format!(
                        "Something bad has happened. Please report this. {x:?}"
                    )))
                }
            };
        }

        Ok(())
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

fn builtin_nodes() -> HashMap<String, (NodeCategory, Arc<NodeCreateFnDyn>)> {
    let mut node_map = HashMap::new();

    // Control nodes
    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Sequence", nodes::control::SequenceNode);
            node.data.children = children;
            node
        },
    ) as Arc<NodeCreateFnDyn>;
    node_map.insert(String::from("Sequence"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "ReactiveSequence",
                nodes::control::ReactiveSequenceNode
            );
            node.data.children = children;
            node
        },
    );
    node_map.insert(
        String::from("ReactiveSequence"),
        (NodeCategory::Control, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "SequenceStar",
                nodes::control::SequenceWithMemoryNode
            );
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("SequenceStar"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Parallel", nodes::control::ParallelNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("Parallel"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "ParallelAll", nodes::control::ParallelAllNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("ParallelAll"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Fallback", nodes::control::FallbackNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("Fallback"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "ReactiveFallback",
                nodes::control::ReactiveFallbackNode
            );
            node.data.children = children;
            node
        },
    );
    node_map.insert(
        String::from("ReactiveFallback"),
        (NodeCategory::Control, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "IfThenElse", nodes::control::IfThenElseNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("IfThenElse"), (NodeCategory::Control, node));

    let node = Arc::new(
        move |config: NodeConfig, children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "WhileDoElse", nodes::control::WhileDoElseNode);
            node.data.children = children;
            node
        },
    );
    node_map.insert(String::from("WhileDoElse"), (NodeCategory::Control, node));

    // Decorator nodes
    // Condition node
    let node = Arc::new(
        move |config: NodeConfig, _children: Vec<TreeNode>| -> TreeNode {
            build_node_ptr!(config, "Condition", nodes::action::ConditionNode)
        },
    );
    node_map.insert(String::from("Condition"), (NodeCategory::Action, node));

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node =
                build_node_ptr!(config, "ForceFailure", nodes::decorator::ForceFailureNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(
        String::from("ForceFailure"),
        (NodeCategory::Decorator, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node =
                build_node_ptr!(config, "ForceSuccess", nodes::decorator::ForceSuccessNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(
        String::from("ForceSuccess"),
        (NodeCategory::Decorator, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Inverter", nodes::decorator::InverterNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("Inverter"), (NodeCategory::Decorator, node));

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(
                config,
                "KeepRunningUntilFailure",
                nodes::decorator::KeepRunningUntilFailureNode
            );
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(
        String::from("KeepRunningUntilFailure"),
        (NodeCategory::Decorator, node),
    );

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Repeat", nodes::decorator::RepeatNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("Repeat"), (NodeCategory::Decorator, node));

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "Retry", nodes::decorator::RetryNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("Retry"), (NodeCategory::Decorator, node));

    let node = Arc::new(
        move |config: NodeConfig, mut children: Vec<TreeNode>| -> TreeNode {
            let mut node = build_node_ptr!(config, "RunOnce", nodes::decorator::RunOnceNode);
            node.data.children = vec![children.remove(0)];
            node
        },
    );
    node_map.insert(String::from("RunOnce"), (NodeCategory::Decorator, node));

    node_map
}
