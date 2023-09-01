use std::{collections::HashMap, any::Any, string::FromUtf8Error, process, sync::Arc, rc::Rc, cell::RefCell};

use quick_xml::{events::{Event, attributes::Attributes}, Reader};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{basic_types::NodeStatus, nodes::{TreeNode, ControlNode, SequenceNode, DummyLeafNode, self, NodeClone, NodeConfig}, blackboard::Blackboard};


#[derive(Debug)]
pub struct Tree {
    // xml: String,
    root: Box<dyn TreeNode>,
}

impl Tree {
    pub fn new(root: Box<dyn TreeNode>) -> Tree {
        Self {
            root
        }
    }

    pub fn tick_while_running(&mut self) -> NodeStatus {
        let mut new_status = self.root.status();

        // Check pre conditions goes here

        new_status = self.root.tick();

        // Check post conditions here

        new_status
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Node {
    DummyNode(DummyNode),
    #[serde(other)]
    Other
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DummyNode {
    #[serde(rename = "@value")]
    pub value: String,
}

#[macro_export]
macro_rules! register_node {
    ($f:ident, $n:expr, $t:ty) => {
        {
            use bt_cpp_rust::nodes::NodeConfig;
    
            let blackboard = $f.blackboard();
            let node_config = NodeConfig::new(blackboard);
            let node = <$t>::new($n, node_config);
            $f.register_node($n, Box::new(node));
        }
    };
    ($f:ident, $n:expr, $t:ty, $($x:expr),*) => {
        <$t>::new($n, $($x),*)
    };
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Port name [{0}] did not match Node [{1}] port list: {2:?}")]
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
    #[error("Placeholder")]
    Empty
}

pub struct Factory {
    node_map: HashMap<String, Box<dyn TreeNode>>,
    blackboard: Rc<RefCell<Blackboard>>,
}

impl Factory {
    pub fn new() -> Factory {
        let mut node_map = HashMap::<String, Box<dyn TreeNode>>::new();
        // node_map.insert("DummyNode".into(), Box::new(DummyLeafNode::new("DummyNode")));

        let blackboard = Rc::new(RefCell::new(Blackboard::new()));

        Self {
            node_map,
            blackboard,
        }
    }

    pub fn set_blackboard(&mut self, blackboard: Rc<RefCell<Blackboard>>) {
        self.blackboard = blackboard;
    }

    pub fn blackboard(&self) -> Rc<RefCell<Blackboard>> {
        Rc::clone(&self.blackboard)
    }

    pub fn register_node(&mut self, name: impl AsRef<str>, node: Box<dyn TreeNode>) {
        self.node_map.insert(name.as_ref().to_string(), node);
    }

    pub fn parse_xml(&self, xml: String) -> Result<Tree, ParseError> {
        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);
    
        let mut buf = Vec::new();

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

        // Try to match first parent tag
        match reader.read_event_into(&mut buf)? {
            Event::Start(e)  => {
                let name = String::from_utf8(e.name().0.into())?;
                return Ok(Tree::new(self.process_node_with_children(&mut reader, name, e.attributes())?));
            }
            _ => return Err(ParseError::MissingRoot)
        };
    }
    
    fn process_node_with_children(&self, reader: &mut Reader<&[u8]>, name: String, attributes: Attributes) -> Result<Box<dyn TreeNode>, ParseError> {
        let mut root_node = match name.as_str() {
            "SequenceNode" => {
                let node_config = NodeConfig::new(Rc::clone(&self.blackboard));
                Box::new(SequenceNode::new(node_config))
            }
            // Add other possibilities here
            _ => return Err(ParseError::UnknownNode(name)),
        };

        // Add ports

        let mut buf = Vec::new();
        
        // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
        loop {
            // NOTE: this is the generic case when we don't know about the input BufRead.
            // when the input is a &str or a &[u8], we don't actually need to use another
            // buffer, we could directly call `reader.read_event()`
            match reader.read_event_into(&mut buf) {
                Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
                Ok(Event::End(_)) => return Ok(root_node),
                // exits the loop when reaching end of file
                Ok(Event::Eof) => {
                    println!("EOF");
                    return Err(ParseError::UnexpectedEof);
                }
                // Node with Children
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8(e.name().0.into())?;

                    let child_node = self.process_node_with_children(reader, name, e.attributes())?;
                    root_node.add_child(child_node);
                }
                // Leaf Node
                Ok(Event::Empty(e)) => {
                    let name = String::from_utf8(e.name().0.into())?;
                    
                    let child_node = match self.node_map.get(&name) {
                        Some(node) => (*node).clone(),
                        None => return Err(ParseError::UnknownNode(name))
                    };

                    let ports = child_node.provided_ports();

                    for attr in e.attributes() {
                        let attr = attr?;
                        let port_name = String::from_utf8(attr.key.0.into())?;
                        if !ports.contains_key(&port_name) {
                            return Err(ParseError::InvalidPort(port_name, name, ports.into_keys().collect()));
                        }

                        let port_value = String::from_utf8(attr.value.to_vec())?;

                        self.blackboard.borrow_mut().write(&port_name, port_value);
                    }

                    root_node.add_child(child_node);
                    
                    // println!("Empty");
                    // println!("- Name: {}", String::from_utf8_lossy(e.name().0));
                    // println!("- Attributes:");
                    // for attr in e.attributes() {
                    //     println!("  - {:?}", attr.unwrap());
                    // }
                }
                Ok(e) => {
                    println!("Other - SHOULDN'T BE HERE");
                    println!("{e:?}");
                },
            }
            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        }
    }
}
