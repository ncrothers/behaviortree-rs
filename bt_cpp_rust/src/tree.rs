use std::{cell::RefCell, collections::HashMap, io::Cursor, rc::Rc, string::FromUtf8Error};

use log::{debug, info};
use quick_xml::{
    events::{attributes::Attributes, Event},
    name::QName,
    Reader,
};
use thiserror::Error;

use crate::{
    basic_types::{AttrsToMap, NodeStatus, PortDirection, PortsRemapping},
    blackboard::{Blackboard, BlackboardPtr},
    macros::build_node_ptr,
    nodes::{
        ActionNodeBase, ControlNodeBase, TreeNodeBase, TreeNodePtr, NodeError, self, DecoratorNodeBase,
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
}

#[derive(Debug)]
pub enum NodePtrType {
    General(Box<dyn TreeNodeBase>),
    Control(Box<dyn ControlNodeBase>),
    Decorator(Box<dyn DecoratorNodeBase>),
    Action(Box<dyn ActionNodeBase>),
}

enum TickOption {
    WhileRunning,
    ExactlyOnce,
    OnceUnlessWokenUp,
}

#[derive(Debug)]
pub struct Tree {
    root: TreeNodePtr,
}

impl Tree {
    pub fn new(root: TreeNodePtr) -> Tree {
        Self { root }
    }

    fn tick_root(&mut self, opt: TickOption) -> Result<NodeStatus, NodeError> {
        let mut status = NodeStatus::Idle;

        while status == NodeStatus::Idle || (matches!(opt, TickOption::WhileRunning) && matches!(status, NodeStatus::Running)) {
            status = self.root.borrow_mut().execute_tick()?;

            // Not implemented: Check for wake-up conditions and tick again if so

            if status.is_completed() {
                self.root.borrow_mut().reset_status();
            }
        }

        Ok(status)
    }

    pub fn tick_exactly_once(&mut self) -> Result<NodeStatus, NodeError> {
        self.tick_root(TickOption::ExactlyOnce)
    }

    pub fn tick_once(&mut self) -> Result<NodeStatus, NodeError> {
        self.tick_root(TickOption::OnceUnlessWokenUp)
    }

    pub fn tick_while_running(&mut self) -> Result<NodeStatus, NodeError> {
        self.tick_root(TickOption::WhileRunning)
    }
}

pub struct Factory {
    node_map: HashMap<String, NodePtrType>,
    blackboard: BlackboardPtr,
    tree_roots: HashMap<String, Reader<Cursor<Vec<u8>>>>,
    main_tree_id: Option<String>,
}

impl Factory {
    pub fn new() -> Factory {
        let blackboard = Rc::new(RefCell::new(Blackboard::new()));

        Self {
            node_map: builtin_nodes(Rc::clone(&blackboard)),
            blackboard,
            tree_roots: HashMap::new(),
            main_tree_id: None,
        }
    }

    pub fn set_blackboard(&mut self, blackboard: BlackboardPtr) {
        self.blackboard = blackboard;
    }

    pub fn blackboard(&self) -> BlackboardPtr {
        Rc::clone(&self.blackboard)
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

    fn recursively_build_subtree(
        &self,
        tree_id: &String,
        tree_name: &String,
        path_prefix: &String,
        blackboard: &BlackboardPtr,
    ) -> Result<TreeNodePtr, ParseError> {
        let mut reader = match self.tree_roots.get(tree_id) {
            Some(root) => root.clone(),
            None => {
                return Err(ParseError::UnknownTree(tree_id.clone()));
            }
        };

        match self.build_child(&mut reader)? {
            Some(child) => Ok(child),
            None => Err(ParseError::NodeTypeMismatch("SubTree".to_string())),
        }
    }

    pub fn create_tree_from_text(&mut self, text: String, blackboard: BlackboardPtr) -> Result<Tree, ParseError> {
        self.register_bt_from_text(text)?;

        if self.tree_roots.len() > 1 && self.main_tree_id.is_none() {
            Err(ParseError::NoMainTree)
        }
        else if self.tree_roots.len() == 1 {
            // Unwrap is safe because we check that tree_roots.len() == 1
            let main_tree_id = self.tree_roots.iter().next().unwrap().0;
    
            self.instantiate_tree(&blackboard, main_tree_id)
        }
        else {
            self.instantiate_tree(&blackboard, self.main_tree_id.as_ref().unwrap())
        }
    }

    pub fn instantiate_tree(
        &self,
        blackboard: &BlackboardPtr,
        main_tree_id: &str,
    ) -> Result<Tree, ParseError> {
        let main_tree_id = String::from(main_tree_id);

        let root_node = self.recursively_build_subtree(
            &main_tree_id,
            &String::new(),
            &String::new(),
            blackboard,
        )?;

        Ok(Tree::new(root_node))
    }

    fn build_leaf_node(
        &self,
        node_name: &String,
        attributes: Attributes,
    ) -> Result<TreeNodePtr, ParseError> {
        // Get clone of node from node_map based on tag name
        let node_ref = self.get_node(node_name)?;

        let node = match node_ref {
            NodePtrType::Action(node) => node.clone(),
            // TODO: expand more
            x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}"))),
        };

        // Get list of defined ports from node

        let node = node.to_tree_node_ptr();

        self.add_ports_to_node(&node, node_name, attributes)?;

        Ok(node)
    }

    fn build_children(
        &self,
        reader: &mut Reader<Cursor<Vec<u8>>>,
    ) -> Result<Vec<TreeNodePtr>, ParseError> {
        let mut nodes = Vec::new();

        while let Some(node) = self.build_child(reader)? {
            nodes.push(node);
        }

        Ok(nodes)
    }

    fn add_ports_to_node(
        &self,
        node_ptr: &TreeNodePtr,
        node_name: &str,
        attributes: Attributes,
    ) -> Result<(), ParseError> {
        let mut node = node_ptr.borrow_mut();
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

    fn build_child(
        &self,
        reader: &mut Reader<Cursor<Vec<u8>>>,
    ) -> Result<Option<TreeNodePtr>, ParseError> {
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

                        let children = self.build_children(reader)?;

                        for child in children {
                            node.add_child(child);
                        }

                        let node = node.to_tree_node_ptr();

                        self.add_ports_to_node(&node, &node_name, attributes)?;

                        node
                    }
                    NodePtrType::Decorator(node) => {
                        let mut node = node.clone();

                        let child = match self.build_child(reader)? {
                            Some(node) => node,
                            None => {
                                return Err(ParseError::NodeTypeMismatch("Decorator".to_string()));
                            }
                        };

                        node.set_child(child);

                        let node = node.to_tree_node_ptr();

                        self.add_ports_to_node(&node, &node_name, attributes)?;

                        node
                    }
                    // TODO: expand more
                    x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}"))),
                };

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
                        match attributes.get("ID") {
                            Some(id) => self.recursively_build_subtree(
                                id,
                                &String::new(),
                                &String::new(),
                                &self.blackboard,
                            )?,
                            None => return Err(ParseError::MissingAttribute("ID".to_string())),
                        }
                    }
                    _ => self.build_leaf_node(&node_name, attributes)?,
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
    }

    pub fn register_bt_from_text(&mut self, xml: String) -> Result<(), ParseError> {
        let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes().to_vec()));
        reader.trim_text(true);

        let mut buf = Vec::new();

        // TODO: Check includes

        // TODO: Parse for correctness

        // Try to match root tag
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                let name = String::from_utf8(e.name().0.into())?;
                let attributes = e.attributes().to_map()?;

                if name.as_str() != "root" {
                    return Err(ParseError::ExpectedRoot(name));
                }
                
                if let Some(tree_id) = attributes.get("main_tree_to_execute") {
                    info!("Found main tree ID: {tree_id}");
                    self.main_tree_id = Some(tree_id.clone());
                }
            }
            _ => return Err(ParseError::MissingRoot),
        }

        buf.clear();

        // Register each BehaviorTree in the XML
        loop {
            let event = { reader.read_event_into(&mut buf)? };

            match event {
                Event::Start(e) => {
                    let name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes().to_map()?;

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

                    let end = e.to_end();
                    let name = end.name();
                    let name = name.as_ref().to_vec().clone();
                    let name = QName(name.as_slice());

                    reader.read_to_end_into(name, &mut buf)?;
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
                    return Err(ParseError::InternalError("Something bad has happened. Please report this.".to_string()))
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

fn builtin_nodes(blackboard: BlackboardPtr) -> HashMap<String, NodePtrType> {
    let mut node_map = HashMap::new();

    // Control nodes
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "Sequence", nodes::control::SequenceNode));
    node_map.insert(String::from("Sequence"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "ReactiveSequence", nodes::control::ReactiveSequenceNode));
    node_map.insert(String::from("ReactiveSequence"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "SequenceStar", nodes::control::SequenceWithMemoryNode));
    node_map.insert(String::from("SequenceStar"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "Parallel", nodes::control::ParallelNode));
    node_map.insert(String::from("Parallel"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "ParallelAll", nodes::control::ParallelAllNode));
    node_map.insert(String::from("ParallelAll"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "Fallback", nodes::control::FallbackNode));
    node_map.insert(String::from("Fallback"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "ReactiveFallback", nodes::control::ReactiveFallbackNode));
    node_map.insert(String::from("ReactiveFallback"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "IfThenElse", nodes::control::IfThenElseNode));
    node_map.insert(String::from("IfThenElse"), node);
    let node = NodePtrType::Control(build_node_ptr!(blackboard, "WhileDoElse", nodes::control::WhileDoElseNode));
    node_map.insert(String::from("WhileDoElse"), node);
    
    // Decorator nodes
    let node = NodePtrType::Decorator(build_node_ptr!(blackboard, "ForceFailure", nodes::decorator::ForceFailureNode));
    node_map.insert(String::from("ForceFailure"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(blackboard, "ForceSuccess", nodes::decorator::ForceSuccessNode));
    node_map.insert(String::from("ForceSuccess"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(blackboard, "Inverter", nodes::decorator::InverterNode));
    node_map.insert(String::from("Inverter"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(blackboard, "KeepRunningUntilFailure", nodes::decorator::KeepRunningUntilFailureNode));
    node_map.insert(String::from("KeepRunningUntilFailure"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(blackboard, "Repeat", nodes::decorator::RepeatNode));
    node_map.insert(String::from("Repeat"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(blackboard, "Retry", nodes::decorator::RetryNode));
    node_map.insert(String::from("Retry"), node);
    let node = NodePtrType::Decorator(build_node_ptr!(blackboard, "RunOnce", nodes::decorator::RunOnceNode));
    node_map.insert(String::from("RunOnce"), node);

    node_map
}
