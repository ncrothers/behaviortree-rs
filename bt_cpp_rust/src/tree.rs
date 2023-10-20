use std::{collections::HashMap, io::Cursor, string::FromUtf8Error};

use futures::future::BoxFuture;
use log::{debug, info};
use quick_xml::{
    events::{attributes::Attributes, Event},
    name::QName,
    Reader,
};
use thiserror::Error;

use crate::{
    basic_types::{
        AttrsToMap, FromString, NodeStatus, ParseBoolError, PortChecks, PortDirection,
        PortsRemapping,
    },
    blackboard::{Blackboard, BlackboardString},
    macros::build_node_ptr,
    nodes::{
        self, ActionNodeBase, ControlNodeBase, DecoratorNodeBase, NodeResult,
        TreeNodeBase, TreeNodePtr, AsyncHalt,
    },
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
}

#[derive(Debug)]
pub enum NodePtrType {
    General(Box<dyn TreeNodeBase + Send + Sync>),
    Control(Box<dyn ControlNodeBase + Send + Sync>),
    Decorator(Box<dyn DecoratorNodeBase + Send + Sync>),
    Action(Box<dyn ActionNodeBase + Send + Sync>),
}

enum TickOption {
    WhileRunning,
    ExactlyOnce,
    OnceUnlessWokenUp,
}

#[derive(Debug)]
pub struct AsyncTree {
    root: TreeNodePtr,
}

impl AsyncTree {
    pub fn new(root: TreeNodePtr) -> AsyncTree {
        Self { root }
    }

    async fn tick_root(&mut self, opt: TickOption) -> NodeResult {
        let mut status = NodeStatus::Idle;

        while status == NodeStatus::Idle
            || (matches!(opt, TickOption::WhileRunning) && matches!(status, NodeStatus::Running))
        {
            status = self.root.lock().await.execute_tick().await?;

            // Not implemented: Check for wake-up conditions and tick again if so

            if status.is_completed() {
                self.root.lock().await.reset_status();
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
        self.root.lock().await.config().blackboard.clone()
    }

    pub async fn halt_tree(&mut self) {
        AsyncHalt::halt(&mut *self.root.lock().await).await;
    }
}

#[derive(Debug)]
pub struct SyncTree {
    root: AsyncTree,
}

impl SyncTree {
    pub fn new(root: TreeNodePtr) -> SyncTree {
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
}

pub struct Factory {
    node_map: HashMap<String, NodePtrType>,
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
            node_map: builtin_nodes(&blackboard),
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

    pub fn register_node(&mut self, name: impl AsRef<str>, node: NodePtrType) {
        self.node_map.insert(name.as_ref().to_string(), node);
    }

    fn get_node(&self, name: &String) -> Result<&NodePtrType, ParseError> {
        match self.node_map.get(name) {
            Some(node) => Ok(node),
            None => Err(ParseError::UnknownNode(name.clone())),
        }
    }

    fn get_uid(&self) -> u32 {
        let uid = *self.tree_uid.lock().unwrap();
        *self.tree_uid.lock().unwrap() += 1;

        uid
    }

    async fn recursively_build_subtree(
        &self,
        tree_id: &String,
        tree_name: &String,
        path_prefix: &String,
        blackboard: Blackboard,
    ) -> Result<TreeNodePtr, ParseError> {
        let mut reader = match self.tree_roots.get(tree_id) {
            Some(root) => root.clone(),
            None => {
                return Err(ParseError::UnknownTree(tree_id.clone()));
            }
        };

        match self
            .build_child(&mut reader, &blackboard, tree_name, path_prefix)
            .await?
        {
            Some(child) => Ok(child),
            None => Err(ParseError::NodeTypeMismatch("SubTree".to_string())),
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

    pub async fn create_async_tree_from_text(
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

            self.instantiate_async_tree(blackboard, &main_tree_id).await
        } else {
            // Unwrap is safe here because there are more than 1 root and
            // self.main_tree_id is Some
            let main_tree_id = self.main_tree_id.clone().unwrap();
            self.instantiate_async_tree(blackboard, &main_tree_id).await
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

        let root_node = futures::executor::block_on(self.recursively_build_subtree(
            &main_tree_id,
            &String::new(),
            &String::new(),
            blackboard,
        ))?;

        Ok(SyncTree::new(root_node))
    }

    pub async fn instantiate_async_tree(
        &mut self,
        blackboard: &Blackboard,
        main_tree_id: &str,
    ) -> Result<AsyncTree, ParseError> {
        // Clone ptr to Blackboard
        let blackboard = blackboard.clone();

        let main_tree_id = String::from(main_tree_id);

        let root_node = self
            .recursively_build_subtree(&main_tree_id, &String::new(), &String::new(), blackboard)
            .await?;

        Ok(AsyncTree::new(root_node))
    }

    async fn build_leaf_node<'a>(
        &self,
        node_name: &String,
        attributes: Attributes<'a>,
        path_prefix: &String,
        blackboard: &Blackboard
    ) -> Result<TreeNodePtr, ParseError> {
        // Get clone of node from node_map based on tag name
        let node_ref = self.get_node(node_name)?;

        let mut node = match node_ref {
            NodePtrType::Action(node) => node.clone(),
            // TODO: expand more
            x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}"))),
        };

        let new_prefix = path_prefix.to_owned() + node_name;

        node.config().path = new_prefix;
        // Set blackboard
        node.config().blackboard = blackboard.clone();

        // Get list of defined ports from node

        let node = node.to_tree_node_ptr();

        self.add_ports_to_node(&node, node_name, attributes).await?;

        Ok(node)
    }

    async fn build_children(
        &self,
        reader: &mut Reader<Cursor<Vec<u8>>>,
        blackboard: &Blackboard,
        tree_name: &String,
        path_prefix: &String,
    ) -> Result<Vec<TreeNodePtr>, ParseError> {
        let mut nodes = Vec::new();

        while let Some(node) = self
            .build_child(reader, blackboard, tree_name, path_prefix)
            .await?
        {
            nodes.push(node);
        }

        Ok(nodes)
    }

    async fn add_ports_to_node<'a>(
        &self,
        node_ptr: &TreeNodePtr,
        node_name: &str,
        attributes: Attributes<'a>,
    ) -> Result<(), ParseError> {
        let mut node = node_ptr.lock().await;
        let config = node.config();
        let manifest = config.manifest()?;

        let mut remap = PortsRemapping::new();

        for (port_name, port_value) in attributes.to_map()? {
            remap.insert(port_name, port_value);
        }

        // Check if all ports from XML match ports in manifest
        for port_name in remap.keys() {
            if !manifest.ports.contains_key(port_name) {
                return Err(ParseError::InvalidPort(
                    port_name.clone(),
                    node_name.to_owned(),
                    manifest.ports.to_owned().into_keys().collect(),
                ));
            }
        }

        // Add ports to NodeConfig
        for (remap_name, remap_val) in remap {
            if let Some(port) = manifest.ports.get(&remap_name) {
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
                    port_info.default_value_str().unwrap(),
                );
            }
        }

        Ok(())
    }

    fn build_child<'a>(
        &'a self,
        reader: &'a mut Reader<Cursor<Vec<u8>>>,
        blackboard: &'a Blackboard,
        tree_name: &'a String,
        path_prefix: &'a String,
    ) -> BoxFuture<Result<Option<TreeNodePtr>, ParseError>> {
        Box::pin(async move {
            let mut buf = Vec::new();

            let node = match reader.read_event_into(&mut buf)? {
                // exits the loop when reaching end of file
                Event::Eof => {
                    debug!("EOF");
                    return Err(ParseError::UnexpectedEof);
                }
                // Node with Children
                Event::Start(e) => {
                    let node_name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes();

                    debug!("build_child Start: {node_name}");

                    let node_ref = self.get_node(&node_name)?;

                    let node = match node_ref {
                        NodePtrType::Control(node) => {
                            let mut node = node.clone();
                            let new_prefix = path_prefix.to_owned() + &node_name;

                            node.config().path = new_prefix;
                            node.config().blackboard = blackboard.clone();

                            let children = self
                                .build_children(
                                    reader,
                                    blackboard,
                                    tree_name,
                                    &(node.config().path.to_owned() + "/"),
                                )
                                .await?;

                            for child in children {
                                node.add_child(child);
                            }

                            let node = node.to_tree_node_ptr();

                            self.add_ports_to_node(&node, &node_name, attributes)
                                .await?;

                            node
                        }
                        NodePtrType::Decorator(node) => {
                            let mut node = node.clone();
                            let new_prefix = path_prefix.to_owned() + &node_name;

                            node.config().path = new_prefix;
                            node.config().blackboard = blackboard.clone();

                            let child = match self
                                .build_child(
                                    reader,
                                    blackboard,
                                    tree_name,
                                    &(node.config().path.to_owned() + "/"),
                                )
                                .await?
                            {
                                Some(node) => node,
                                None => {
                                    return Err(ParseError::NodeTypeMismatch(
                                        "Decorator".to_string(),
                                    ));
                                }
                            };

                            node.set_child(child);

                            let node = node.to_tree_node_ptr();

                            self.add_ports_to_node(&node, &node_name, attributes)
                                .await?;

                            node
                        }
                        // TODO: expand more
                        x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}"))),
                    };

                    // Advance pointer one time to skip the end tag
                    let mut buf = Vec::new();
                    reader.read_event_into(&mut buf)?;

                    Some(node)
                }
                // Leaf Node
                Event::Empty(e) => {
                    let node_name = String::from_utf8(e.name().0.into())?;
                    debug!("[Leaf node]: {node_name}");
                    let attributes = e.attributes();

                    let node = match node_name.as_str() {
                        "SubTree" => {
                            let attributes = attributes.to_map()?;
                            let mut child_blackboard = Blackboard::with_parent(blackboard).await;

                            // Process attributes (Ports, special fields, etc)
                            for (attr, value) in attributes.iter() {
                                // Set autoremapping to true or false
                                if attr == "_autoremap" {
                                    child_blackboard
                                        .enable_auto_remapping(<bool as FromString>::from_string(
                                            value,
                                        )?)
                                        .await;
                                    continue;
                                } else if !attr.is_allowed_port_name() {
                                    continue;
                                }

                                if let Some(port_name) = value.strip_bb_pointer() {
                                    // Add remapping if `value` is a Blackboard pointer
                                    child_blackboard
                                        .add_subtree_remapping(attr.clone(), port_name)
                                        .await;
                                } else {
                                    // Set string value into Blackboard
                                    child_blackboard.set(attr, value.clone()).await;
                                }
                            }

                            let id = match attributes.get("ID") {
                                Some(id) => id,
                                None => return Err(ParseError::MissingAttribute("ID".to_string())),
                            };

                            let mut subtree_name = tree_name.clone();
                            if !subtree_name.is_empty() {
                                subtree_name += "/";
                            }

                            if let Some(name_attr) = attributes.get("name") {
                                subtree_name += name_attr;
                            } else {
                                subtree_name += &format!("{id}::{}", self.get_uid());
                            }

                            let new_prefix = format!("{subtree_name}/");

                            self.recursively_build_subtree(
                                id,
                                &subtree_name,
                                &new_prefix,
                                child_blackboard,
                            )
                            .await?
                        }
                        _ => {
                            self.build_leaf_node(&node_name, attributes, path_prefix, blackboard)
                                .await?
                        }
                    };

                    Some(node)
                }
                Event::End(_e) => None,
                e => {
                    debug!("Other - SHOULDN'T BE HERE");
                    debug!("{e:?}");

                    return Err(ParseError::InternalError(
                        "Didn't match one of the expected XML tag types.".to_string(),
                    ));
                }
            };

            Ok(node)
        })
    }

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
                    let name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes().to_map()?;

                    // Strange method of cloning QName such that the internal buffer is also cloned
                    // Otherwise, borrow checker errors with &mut buf still being borrowed
                    let end = e.to_end();
                    let end_name = end.name().as_ref().to_vec().clone();
                    let end_name = QName(end_name.as_slice());

                    // TODO: Maybe do something with TreeNodesModel?
                    // For now, just ignore it
                    if name.as_str() == "TreeNodesModel" {
                        reader.read_to_end_into(end_name, &mut buf)?;
                    } else {
                        // Add error for missing BT
                        if name.as_str() != "BehaviorTree" {
                            return Err(ParseError::ExpectedRoot(name));
                        }

                        // Save position of Reader for each BT
                        if let Some(id) = attributes.get("ID") {
                            self.tree_roots.insert(id.clone(), reader.clone());
                        } else {
                            return Err(ParseError::MissingAttribute("Found BehaviorTree definition without ID. Cannot continue parsing.".to_string()));
                        }

                        reader.read_to_end_into(end_name, &mut buf)?;
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
                _ => {
                    return Err(ParseError::InternalError(
                        "Something bad has happened. Please report this.".to_string(),
                    ))
                }
            };
        }

        buf.clear();

        Ok(())
    }
}

impl Default for Factory {
    fn default() -> Self {
        Self::new()
    }
}

fn builtin_nodes(blackboard: &Blackboard) -> HashMap<String, NodePtrType> {
    let mut node_map = HashMap::new();

    // Control nodes
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "Sequence",
        nodes::control::SequenceNode
    ));
    node_map.insert(String::from("Sequence"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "ReactiveSequence",
        nodes::control::ReactiveSequenceNode
    ));
    node_map.insert(String::from("ReactiveSequence"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "SequenceStar",
        nodes::control::SequenceWithMemoryNode
    ));
    node_map.insert(String::from("SequenceStar"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "Parallel",
        nodes::control::ParallelNode
    ));
    node_map.insert(String::from("Parallel"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "ParallelAll",
        nodes::control::ParallelAllNode
    ));
    node_map.insert(String::from("ParallelAll"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "Fallback",
        nodes::control::FallbackNode
    ));
    node_map.insert(String::from("Fallback"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "ReactiveFallback",
        nodes::control::ReactiveFallbackNode
    ));
    node_map.insert(String::from("ReactiveFallback"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "IfThenElse",
        nodes::control::IfThenElseNode
    ));
    node_map.insert(String::from("IfThenElse"), node);
    let node = NodePtrType::Control(build_node_ptr!(
        blackboard,
        "WhileDoElse",
        nodes::control::WhileDoElseNode
    ));
    node_map.insert(String::from("WhileDoElse"), node);

    // Decorator nodes
    let node = NodePtrType::Decorator(build_node_ptr!(
        blackboard,
        "ForceFailure",
        nodes::decorator::ForceFailureNode
    ));
    node_map.insert(String::from("ForceFailure"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(
        blackboard,
        "ForceSuccess",
        nodes::decorator::ForceSuccessNode
    ));
    node_map.insert(String::from("ForceSuccess"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(
        blackboard,
        "Inverter",
        nodes::decorator::InverterNode
    ));
    node_map.insert(String::from("Inverter"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(
        blackboard,
        "KeepRunningUntilFailure",
        nodes::decorator::KeepRunningUntilFailureNode
    ));
    node_map.insert(String::from("KeepRunningUntilFailure"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(
        blackboard,
        "Repeat",
        nodes::decorator::RepeatNode
    ));
    node_map.insert(String::from("Repeat"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(
        blackboard,
        "Retry",
        nodes::decorator::RetryNode
    ));
    node_map.insert(String::from("Retry"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(
        blackboard,
        "RunOnce",
        nodes::decorator::RunOnceNode
    ));
    node_map.insert(String::from("RunOnce"), node);

    node_map
}
