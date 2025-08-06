use crate::{
    basic_types::NodeStatus,
    blackboard::Blackboard,
    error::ParseError,
    node_registry::NodeRegistry,
    nodes::{NodeResult, TreeNode},
    parser::Parser,
};

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
/// #[derive(Debug)]
/// struct SimpleNode;
///
/// impl SyncActionNode for SimpleNode {
///     fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
///         Ok(NodeStatus::Success)
///     }
/// }
///
/// let mut registry = NodeRegistry::default();
///
/// // Insert custom nodes into the registry here
/// registry.insert("SimpleNode", || SimpleNode.to_boxed(), NodeType::Action);
///
/// let xml = r#"
/// <root>
///     <BehaviorTree ID="main-tree">
///         <SimpleNode />
///     </BehaviorTree>
/// </root>
/// "#;
///
/// let tree = Tree::builder(xml, &registry)
///     // Optional
///     .tree_name("main-tree")
///     // Optional
///     .blackboard(Blackboard::new())
///     .build();
///
/// assert!(tree.is_ok());
/// ```
pub struct TreeBuilder<'a> {
    /// Holds all registered nodes
    pub(crate) registry: &'a NodeRegistry,
    /// XML text to parse the tree from
    pub(crate) xml: &'a str,
    /// Optional. Specify which tree to build by ID.
    ///
    /// When this is `None`, the `main_tree_to_execute` value will be used if set.
    /// If there is no `main_tree_to_execute` attribute set, there must be only
    /// one behavior tree defined in the XML text, otherwise the build will fail.
    pub(crate) tree_name: Option<&'a str>,
    /// Optional. Provide an external [`Blackboard`] to use as the root for this
    /// tree. If not provided, a new one will be allocated and used as the root.
    pub(crate) blackboard: Option<Blackboard>,
}

pub(crate) struct TreeConfig<'a> {
    pub(crate) registry: &'a NodeRegistry,
    pub(crate) xml: &'a str,
    pub(crate) tree_name: Option<&'a str>,
    pub(crate) blackboard: Blackboard,
}

/// Top-level container of a behavior tree. Provides methods to tick the tree,
/// access the root-level [`Blackboard`], and iterate over its nodes.
///
/// ```
/// use behaviortree_rs::prelude::*;
///
/// #[derive(Debug)]
/// struct SimpleNode;
///
/// impl SyncActionNode for SimpleNode {
///     fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
///         Ok(NodeStatus::Success)
///     }
/// }
///
/// let mut registry = NodeRegistry::default();
///
/// // Insert custom nodes into the registry here
/// registry.insert("SimpleNode", || SimpleNode.to_boxed(), NodeType::Action);
///
/// // Insert custom nodes into the registry here
///
/// let xml = r#"
/// <root>
///     <BehaviorTree ID="main-tree">
///         <SimpleNode />
///     </BehaviorTree>
/// </root>
/// "#;
///
/// let tree = Tree::builder(xml, &registry)
///     // Optional
///     .tree_name("main-tree")
///     // Optional
///     .blackboard(Blackboard::default())
///     .build();
///
/// assert!(tree.is_ok());
/// ```
#[derive(Debug)]
pub struct Tree {
    root: TreeNode,
}

impl<'a> TreeBuilder<'a> {
    pub fn tree_name(self, name: &'a str) -> Self {
        TreeBuilder {
            tree_name: Some(name),
            ..self
        }
    }

    pub fn blackboard(self, blackboard: Blackboard) -> Self {
        TreeBuilder {
            blackboard: Some(blackboard),
            ..self
        }
    }

    pub fn build(&self) -> Result<Tree, ParseError> {
        let config = TreeConfig {
            xml: self.xml,
            registry: self.registry,
            tree_name: self.tree_name,
            blackboard: self
                .blackboard
                .as_ref()
                .map(|bb| bb.clone())
                .unwrap_or_default(),
        };

        let mut parser = Parser::new(&config);

        parser.create_tree()
    }
}

impl Tree {
    pub(crate) fn new(root: TreeNode) -> Tree {
        Self { root }
    }

    pub fn builder<'a>(xml: &'a str, node_registry: &'a NodeRegistry) -> TreeBuilder<'a> {
        TreeBuilder {
            registry: node_registry,
            xml,
            tree_name: None,
            blackboard: None,
        }
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
