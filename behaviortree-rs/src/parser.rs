use std::{cell::RefCell, collections::HashMap, io::Cursor, str::FromStr, sync::Arc};

use quick_xml::{events::Event, name::QName, Reader};

use crate::{
    basic_types::{is_allowed_port_name, AttrsToMap, NodeStatus, NodeType, TreeNodeManifest},
    blackboard::{Blackboard, BlackboardString},
    error::ParseError,
    nodes::{decorator::SubTreeNode, NodeDataGeneric, NodeMetadata, ToBoxed, TreeNode},
    tree::{Tree, TreeConfig},
};

#[derive(Debug)]
enum CreateNodeControl {
    Node(TreeNode),
    Continue,
    End,
}

/// Parses a behavior tree XML into a [`Tree`] based on the provided [`TreeConfig`].
pub(crate) struct Parser<'a> {
    /// Registry of tree nodes, both built-in and custom-defined
    tree_config: &'a TreeConfig<'a>,
    /// Mapping from each `<BehaviorTree>`'s `ID` to a copy of the `Reader`
    /// at the start of the tree, which can then be parsed and loaded.
    tree_roots: HashMap<String, Reader<Cursor<Vec<u8>>>>,
    /// After registering an XML, if it finds a tree that can be the "entrypoint",
    /// it sets it here.
    ///
    /// This value will be set in two situations:
    ///
    /// * The `main_tree_to_execute` attribute is set on the `<root>` tag
    /// * There is only one `<BehaviorTree` element in the text.
    main_tree_id: Option<String>,

    // TODO: temporary solution, potentially replace later
    tree_uid: RefCell<u32>,
}

impl<'a> Parser<'a> {
    /// Creates a new [`Factory`] which has all of the built-in nodes already
    /// registered.
    pub fn new(tree_config: &'a TreeConfig<'a>) -> Self {
        Self {
            tree_config,
            tree_roots: HashMap::new(),
            main_tree_id: None,
            tree_uid: RefCell::new(0),
        }
    }

    fn get_uid(&self) -> u32 {
        self.tree_uid.replace_with(|prev| *prev + 1)
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
                CreateNodeControl::Node(child) => break Ok(child),
                CreateNodeControl::Continue => (),
                CreateNodeControl::End => {
                    break Err(ParseError::NodeTypeMismatch("SubTree".to_string()))
                }
            }
        }
    }

    /// Builds and returns a [`Tree`], parsing the `text` as XML. Will return
    /// a [`ParseError`] if it's unable to build the tree.
    ///
    /// For this method to work, one of the following must be true:
    ///
    /// * The `main_tree_to_execute` attribute is set on the `<root>` tag
    /// * There is only one `<BehaviorTree` element in the text.
    pub(crate) fn create_tree(&mut self) -> Result<Tree, ParseError> {
        self.register_bt_from_text(self.tree_config.xml)?;

        // Get which tree ID to instantiate
        // If there are multiple trees defined and no name specified, return error
        let tree_id = if self.tree_roots.len() > 1
            && self.main_tree_id.is_none()
            && self.tree_config.tree_name.is_none()
        {
            return Err(ParseError::NoMainTree);
        }
        // If there's exactly one tree, that tree is necessarily _always_ the correct tree
        // However, handle user-specified tree name separately
        else if self.tree_roots.len() == 1
            && self.main_tree_id.is_none()
            && self.tree_config.tree_name.is_none()
        {
            // Unwrap is safe because we check that tree_roots.len() == 1
            self.tree_roots.iter().next().unwrap().0.clone()
        }
        // Handle a user-provided tree name separately so we return an error if the specified
        // tree does not exist
        else if self.tree_config.tree_name.is_some() {
            // Unwrap is safe because this is only reachable if tree_name is Some
            self.tree_config.tree_name.unwrap().to_string()
        }
        // Handle a tree name defined in the XML root tag
        else if self.main_tree_id.is_some() {
            // Unwrap is safe because this is only reachable if main_tree_id is Some
            self.main_tree_id.clone().unwrap()
        }
        // Multiple roots and main_tree_id is Some
        else if self.tree_roots.len() > 1 && self.main_tree_id.is_some() {
            self.main_tree_id.clone().unwrap()
        }
        // Only reachable if there are no BehaviorTree elements
        else {
            return Err(ParseError::NoMainTree);
        };

        self.instantiate_tree(&self.tree_config.blackboard, &tree_id)
    }

    /// Builds and returns the [`Tree`] with the name `main_tree_id`. This method
    /// can only be called after calling [`Factory::register_bt_from_text`], which
    /// loads all of the `<BehaviorTree>` definitions in the XML.
    fn instantiate_tree(
        &mut self,
        blackboard: &Blackboard,
        main_tree_id: &str,
    ) -> Result<Tree, ParseError> {
        // Clone ptr to Blackboard
        let blackboard = blackboard.clone();

        let root_node = self.recursively_build_subtree(main_tree_id, "", "", blackboard)?;

        Ok(Tree::new(root_node))
    }

    /// Parses `xml` as XML and loads all of the `<BehaviorTree>` elements in
    /// preparation for building [`Tree`]s
    fn register_bt_from_text(&mut self, xml: &str) -> Result<(), ParseError> {
        let mut reader = Reader::from_reader(Cursor::new(xml.as_bytes().to_vec()));
        reader.config_mut().trim_text(true);

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
                        log::debug!("Found main tree ID: {tree_id}");
                        self.main_tree_id = Some(tree_id.clone());
                    }

                    buf.clear();
                    break;
                }
                _ => return Err(ParseError::MissingRoot),
            }
        }

        self.register_trees(&mut reader, &mut buf)?;

        Ok(())
    }

    /// Continues building children until none left, returning the set of children up
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
                CreateNodeControl::Node(node) => {
                    nodes.push(node);
                }
                CreateNodeControl::Continue => (),
                CreateNodeControl::End => break,
            }
        }

        Ok(nodes)
    }

    fn add_ports_to_node(
        &self,
        node_ptr: &mut TreeNode,
        node_name: &str,
        attributes: HashMap<String, String>,
    ) -> Result<(), ParseError> {
        let manifest = Arc::clone(&node_ptr.data.meta.manifest);

        let remap = attributes;

        // Check if all ports from XML match ports in manifest
        for port_name in remap.keys() {
            if !manifest.ports.contains_key(port_name) {
                return Err(ParseError::InvalidPort(
                    port_name.clone(),
                    node_name.to_owned(),
                    manifest.ports.keys().cloned().collect(),
                ));
            }
        }

        // Set string port values
        for (remap_name, remap_val) in remap {
            if let Some(port) = manifest.ports.get(&remap_name) {
                // Validate that any expr-enabled ports contain valid expressions,
                // and the provided types for blackboard pointers are one of the valid ones
                #[cfg(feature = "expr")]
                if port.parse_expr() {
                    let expr =
                        evalexpr::build_operator_tree::<evalexpr::DefaultNumericTypes>(&remap_val)?;

                    for key in expr.iter_variable_identifiers() {
                        // Check if it's a blackboard pointer
                        if let Some(inner_key) = key.strip_bb_pointer() {
                            // Split the type from the name
                            let (name, var_type) = inner_key.split_once(':').ok_or_else(|| {
                                ParseError::PortExpressionMissingType(inner_key.to_owned())
                            })?;

                            // Check if the type is supported
                            match var_type {
                                "int" | "float" | "str" | "bool" => (),
                                _ => {
                                    return Err(ParseError::PortExpressionInvalidType {
                                        ident: name.to_owned(),
                                        type_name: var_type.to_owned(),
                                    })
                                }
                            };
                        }
                    }
                }

                node_ptr
                    .data
                    .meta
                    .set_port_value(port.direction(), remap_name, remap_val);
            }
        }

        // Check if unspecified port values have a default. If not, they are required
        // ports and parsing should fail
        for (port_name, port_info) in manifest.ports.iter() {
            let direction = port_info.direction();

            // If the port value hasn't been set and it doesn't have a default
            // value, return error
            if !(node_ptr.data.meta.is_port_value_set(port_name, direction)
                || port_info.has_default())
            {
                return Err(ParseError::MissingRequiredPort {
                    node: node_name.to_string(),
                    port: port_name.clone(),
                });
            }
        }

        Ok(())
    }

    fn build_child<'b>(
        &'b self,
        reader: &'b mut Reader<Cursor<Vec<u8>>>,
        blackboard: &'b Blackboard,
        tree_name: &'b str,
        path_prefix: &'b str,
    ) -> Result<CreateNodeControl, ParseError> {
        let mut buf = Vec::new();

        let node = match reader.read_event_into(&mut buf)? {
            // exits the loop when reaching end of file
            Event::Eof => {
                log::debug!("EOF");
                return Err(ParseError::UnexpectedEof);
            }
            // Node with Children
            Event::Start(e) => {
                let node_name = String::from_utf8(e.name().0.into())?;
                let attributes = e.attributes().to_map()?;

                log::debug!("build_child Start: {node_name}");

                let path = path_prefix.to_owned() + &node_name;

                let (node_type, node) = self
                    .tree_config
                    .registry
                    .build_node(&node_name)
                    .ok_or_else(|| ParseError::UnknownNode(node_name.clone()))?;

                let node_meta = NodeMetadata::new(
                    node_name.clone(),
                    path.clone(),
                    node_type,
                    Arc::new(TreeNodeManifest::new(
                        node_type,
                        node_name.clone(),
                        node.ports(),
                        String::new(),
                    )),
                );

                let node = match node_type {
                    NodeType::Control => {
                        let children =
                            self.build_children(reader, blackboard, tree_name, &(path + "/"))?;

                        if children.is_empty() {
                            return Err(ParseError::ViolateNodeConstraint(
                                "Control nodes must have at least one child".into(),
                            ));
                        }

                        let node_data = NodeDataGeneric {
                            meta: node_meta,
                            status: NodeStatus::Idle,
                            children,
                            blackboard: blackboard.clone(),
                        };

                        let mut node = TreeNode {
                            data: node_data,
                            node,
                        };

                        self.add_ports_to_node(&mut node, &node_name, attributes.clone())?;

                        node
                    }
                    NodeType::Decorator => {
                        // Loop until either an end tag or the child is found
                        let child = loop {
                            match self.build_child(
                                reader,
                                blackboard,
                                tree_name,
                                &(path.clone() + "/"),
                            )? {
                                CreateNodeControl::Node(node) => break node,
                                CreateNodeControl::Continue => (),
                                CreateNodeControl::End => {
                                    return Err(ParseError::NodeTypeMismatch(
                                        "Decorator".to_string(),
                                    ))
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

                        let node_data = NodeDataGeneric {
                            meta: node_meta,
                            status: NodeStatus::Idle,
                            children: vec![child],
                            blackboard: blackboard.clone(),
                        };

                        let mut node = TreeNode {
                            data: node_data,
                            node,
                        };

                        self.add_ports_to_node(&mut node, &node_name, attributes)?;

                        node
                    }
                    // TODO: expand more
                    x => return Err(ParseError::NodeTypeMismatch(format!("{x:?}"))),
                };

                CreateNodeControl::Node(node)
            }
            // Leaf Node
            Event::Empty(e) => {
                let node_name = String::from_utf8(e.name().0.into())?;
                log::debug!("[Leaf node]: {node_name}");
                let attributes = e.attributes().to_map()?;

                let node = match node_name.as_str() {
                    "SubTree" => {
                        let child_blackboard = Blackboard::with_parent(blackboard);

                        // Process attributes (Ports, special fields, etc)
                        for (attr, value) in attributes.iter() {
                            // Set autoremapping to true or false
                            if attr == "_autoremap" {
                                child_blackboard
                                    .set_auto_remapping(<bool as FromStr>::from_str(value)?);
                                continue;
                            } else if !is_allowed_port_name(attr) {
                                continue;
                            }

                            // Add remapping if `value` is a Blackboard pointer
                            if let Some(port_name) = value.strip_bb_pointer() {
                                child_blackboard
                                    .add_subtree_remapping(attr.to_owned(), port_name.to_string());
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

                        let path = path_prefix.to_owned() + id;

                        let child = self.recursively_build_subtree(
                            id,
                            &subtree_name,
                            &new_prefix,
                            child_blackboard,
                        )?;

                        let node = SubTreeNode.to_boxed();

                        let node_meta = NodeMetadata::new(
                            id.to_owned(),
                            path,
                            NodeType::SubTree,
                            Arc::new(TreeNodeManifest::new(
                                NodeType::Control,
                                node_name.clone(),
                                node.ports(),
                                String::new(),
                            )),
                        );

                        let node_data = NodeDataGeneric {
                            meta: node_meta,
                            status: NodeStatus::Idle,
                            children: vec![child],
                            blackboard: blackboard.clone(),
                        };

                        TreeNode {
                            data: node_data,
                            node,
                        }
                    }
                    _ => {
                        // Get clone of node from node_map based on tag name
                        let (node_type, node) = self
                            .tree_config
                            .registry
                            .build_node(&node_name)
                            .ok_or_else(|| ParseError::UnknownNode(node_name.clone()))?;

                        if !matches!(node_type, NodeType::Action) {
                            return Err(ParseError::NodeTypeMismatch(String::from("Action")));
                        }

                        let path = path_prefix.to_owned() + &node_name;

                        let node_meta = NodeMetadata::new(
                            node_name.clone(),
                            path,
                            node_type,
                            Arc::new(TreeNodeManifest::new(
                                node_type,
                                node_name.clone(),
                                node.ports(),
                                String::new(),
                            )),
                        );

                        let node_data = NodeDataGeneric {
                            meta: node_meta,
                            status: NodeStatus::Idle,
                            children: Vec::new(),
                            blackboard: blackboard.clone(),
                        };

                        let mut node = TreeNode {
                            data: node_data,
                            node,
                        };

                        self.add_ports_to_node(&mut node, &node_name, attributes)?;

                        node
                    }
                };

                CreateNodeControl::Node(node)
            }
            Event::End(_e) => CreateNodeControl::End,
            Event::Comment(content) => {
                log::debug!("Comment - \"{content:?}\"");
                CreateNodeControl::Continue
            }
            e => {
                log::debug!("Other - SHOULDN'T BE HERE");
                log::debug!("{e:?}");

                return Err(ParseError::InternalError(
                    "Didn't match one of the expected XML tag types.".to_string(),
                ));
            }
        };

        Ok(node)
    }

    fn register_trees(
        &mut self,
        reader: &mut Reader<Cursor<Vec<u8>>>,
        buf: &mut Vec<u8>,
    ) -> Result<(), ParseError> {
        // Register each BehaviorTree in the XML
        loop {
            let event = reader.read_event_into(buf)?;

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
                        reader.read_to_end_into(end_name, buf)?;
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

                        let mut buf = Vec::new();

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
