use std::{collections::HashMap, string::FromUtf8Error, rc::Rc, cell::RefCell, io::Cursor};

use log::{debug, info};
use quick_xml::{events::{Event, attributes::Attributes}, Reader, name::QName};
use thiserror::Error;

use crate::{basic_types::{NodeStatus, TreeNodeManifest, PortDirection, PortsRemapping, AttrsToMap}, nodes::{TreeNode, SequenceNode, NodeConfig, TreeNodeBase, GetNodeType, TreeNodeDefaults, ControlNodeBase, ActionNodeBase, TreeNodePtr}, blackboard::Blackboard};

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
}

#[derive(Debug)]
pub enum NodePtrType {
    General(Box<dyn TreeNodeBase>),
    Control(Box<dyn ControlNodeBase>),
    Decorator(Box<dyn ControlNodeBase>),
    Action(Box<dyn ActionNodeBase>),
}

#[derive(Debug)]
pub struct Tree {
    root: TreeNodePtr,
}

impl Tree {
    pub fn new(root: TreeNodePtr) -> Tree {
        Self {
            root
        }
    }

    pub fn tick_while_running(&mut self) -> NodeStatus {
        let mut new_status = self.root.borrow().status();

        // Check pre conditions goes here

        new_status = self.root.borrow_mut().tick();

        // Check post conditions here

        new_status
    }
}

pub struct Factory {
    node_map: HashMap<String, NodePtrType>,
    blackboard: Rc<RefCell<Blackboard>>,
    tree_roots: HashMap<String, Reader<Cursor<Vec<u8>>>>,
}

impl Factory {
    pub fn new() -> Factory {
        let blackboard = Rc::new(RefCell::new(Blackboard::new()));
        let mut node_map = HashMap::new();

        let config = NodeConfig::new(Rc::clone(&blackboard));
        let mut node = SequenceNode::new(config);
        let manifest = TreeNodeManifest {
            node_type: node.node_type(),
            registration_id: "SequenceNode".to_string(),
            ports: node.provided_ports(),
            description: String::new(),
        };
        node.config().set_manifest(Rc::new(manifest));
        node_map.insert(String::from("SequenceNode"), NodePtrType::Control(Box::new(node)));

        Self {
            node_map: node_map,
            blackboard: blackboard,
            tree_roots: HashMap::new(),
        }
    }

    pub fn set_blackboard(&mut self, blackboard: Rc<RefCell<Blackboard>>) {
        self.blackboard = blackboard;
    }

    pub fn blackboard(&self) -> Rc<RefCell<Blackboard>> {
        Rc::clone(&self.blackboard)
    }

    pub fn register_node(&mut self, name: impl AsRef<str>, node: NodePtrType) {
        self.node_map.insert(name.as_ref().to_string(), node);
    }

    fn get_node(&self, name: &String) -> Result<&NodePtrType, ParseError> {
        match self.node_map.get(name) {
            Some(node) => Ok(node),
            None => Err(ParseError::UnknownNode(name.clone()))
        }
    }

    fn recursively_build_subtree(&self, tree_id: &String, tree_name: &String, path_prefix: &String, blackboard: &Rc<RefCell<Blackboard>>) -> Result<TreeNodePtr, ParseError> {
        let mut reader = match self.tree_roots.get(tree_id) {
            Some(root) => root.clone(),
            None => {
                return Err(ParseError::UnknownTree(tree_id.clone()));
            }
        };

        match self.build_child(&mut reader)? {
            Some(child) => Ok(child),
            None => Err(ParseError::NodeTypeMismatch(format!("SubTree")))
        }
    }

    pub fn instantiate_tree(&self, blackboard: &Rc<RefCell<Blackboard>>, main_tree_id: &str) -> Result<Tree, ParseError> {
        let main_tree_id = String::from(main_tree_id);

        let root_node = self.recursively_build_subtree(&main_tree_id, &String::new(), &String::new(), blackboard)?;
        
        Ok(Tree::new(root_node))
    }

    fn build_leaf_node(&self, node_name: &String, attributes: Attributes) -> Result<TreeNodePtr, ParseError> {
        // Get clone of node from node_map based on tag name
        let node_ref = self.get_node(node_name)?;

        let mut node = match node_ref {
            NodePtrType::Action(node) => {
                node.clone()
            }
            // TODO: expand more
            x @ _ => return Err(ParseError::NodeTypeMismatch(format!("{x:?}")))
        };

        // Get list of defined ports from node

        let config = node.config();
        let manifest = config.manifest()?;

        let mut remap = PortsRemapping::new();

        for attr in attributes {
            let attr = attr?;
            let port_name = String::from_utf8(attr.key.0.into())?;
            let port_value = String::from_utf8(attr.value.to_vec())?;

            remap.insert(port_name, port_value);
        }

        // Check if all ports from XML match ports in manifest
        for port_name in remap.keys() {
            if !manifest.ports.contains_key(port_name) {
                return Err(ParseError::InvalidPort(port_name.clone(), node_name.clone(), manifest.ports.to_owned().into_keys().into_iter().collect()));
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
                && !config.has_port(direction, &port_name) 
                && port_info.default_value().is_some()
            {
                config.add_port(PortDirection::Input, port_name.clone(), port_info.default_value_str().unwrap());
            }
        }

        Ok(node.into_tree_node_ptr())
    }

    fn build_children(&self, reader: &mut Reader<Cursor<Vec<u8>>>) -> Result<Vec<TreeNodePtr>, ParseError> {
        let mut nodes = Vec::new();

        while let Some(node) = self.build_child(reader)? {
            nodes.push(node);
        }

        Ok(nodes)
    }

    fn build_child(&self, reader: &mut Reader<Cursor<Vec<u8>>>) -> Result<Option<TreeNodePtr>, ParseError> {

        let blackboard = Rc::clone(&self.blackboard);
        let config = NodeConfig::new(blackboard);
        let test = NodePtrType::General(Box::new(SequenceNode::new(config)));

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

                debug!("build_child Start: {node_name}");

                let node_ref = self.get_node(&node_name)?;

                let node = match node_ref {
                    NodePtrType::Control(node) => {
                        let mut node = node.clone();

                        let children = self.build_children(reader)?;

                        for child in children {
                            node.add_child(child);
                        }

                        node.into_tree_node_ptr()
                    }
                    NodePtrType::Decorator(node) => {
                        let mut node = node.clone();

                        let child = match self.build_child(reader)? {
                            Some(node) => node,
                            None => {
                                return Err(ParseError::NodeTypeMismatch(format!("Decorator")));
                            }
                        };

                        node.add_child(child);

                        node.into_tree_node_ptr()
                    }
                    // TODO: expand more
                    x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}")))
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
                            Some(id) => self.recursively_build_subtree(id, &String::new(), &String::new(), &self.blackboard)?,
                            None => return Err(ParseError::MissingAttribute("ID".to_string()))
                        }
                    }
                    _ => self.build_leaf_node(&node_name, attributes)?
                };

                Some(node)
            }
            Event::End(e) => {
                None
            }
            e => {
                debug!("Other - SHOULDN'T BE HERE");
                debug!("{e:?}");

                return Err(ParseError::InternalError("Didn't match one of the expected XML tag types.".to_string()));
            },
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
            Event::Start(e)  => {
                let name = String::from_utf8(e.name().0.into())?;
                if name.as_str() != "root" {
                    return Err(ParseError::ExpectedRoot(name));
                }
            }
            _ => return Err(ParseError::MissingRoot)
        }

        buf.clear();

        // Register each BehaviorTree in the XML
        loop {
            let event = {
                reader.read_event_into(&mut buf)?
            };

            match event {
                Event::Start(e)  => {
                    let name = String::from_utf8(e.name().0.into())?;
                    let attributes = e.attributes().to_map()?;
                    
                    // Add error for missing BT
                    if name.as_str() != "BehaviorTree" {
                        return Err(ParseError::ExpectedRoot(name));
                    }
                    
                    // Save position of Reader for each BT
                    if let Some(id) = attributes.get("ID") {
                        self.tree_roots.insert(id.clone(), reader.clone());
                    }
                    else {
                        return Err(ParseError::MissingAttribute(format!("Found BehaviorTree definition without ID. Cannot continue parsing.")));
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
                        return Err(ParseError::InternalError(format!("A non-root end tag was found. This should not happen. Please report this.")))
                    }
                    else {
                        break;
                    }
                }
                _ => return Err(ParseError::InternalError(format!("Something bad has happened. Please report this.")))
            };
        }


        buf.clear();

        Ok(())
    }

}
