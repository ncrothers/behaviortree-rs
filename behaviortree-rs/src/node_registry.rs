use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use crate::{
    basic_types::NodeType,
    nodes::{self, NodeBase, ToBoxed},
};

type NodeCreateFnDyn = dyn Fn() -> Box<dyn NodeBase> + Send + Sync;

static BUILTIN_NODES: OnceLock<HashMap<String, (NodeType, Arc<NodeCreateFnDyn>)>> = OnceLock::new();

/// Registry of nodes that are available when parsing and building a [`Tree`].
/// Should be passed to a [`TreeConfig`](crate::prelude::TreeConfig).
///
/// When creating multiple [`Tree`]s, it is highly recommended to reuse a single
/// `NodeRegistry` rather than building a new one for each tree.
///
/// [`Tree`]: crate::prelude::Tree
///
/// # Examples
///
/// ```
/// use behaviortree_rs::prelude::*;
///
/// #[derive(Debug)]
/// struct CustomNode;
///
/// impl SyncActionNode for CustomNode {
///     fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
///         Ok(NodeStatus::Success)
///     }
/// }
///
/// let mut registry = NodeRegistry::new();
///
/// registry.insert("CustomNode", || CustomNode.to_boxed(), NodeType::Action);
/// ```
#[derive(Default)]
pub struct NodeRegistry {
    node_map: HashMap<String, (NodeType, Arc<NodeCreateFnDyn>)>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            node_map: HashMap::new(),
        }
    }

    /// Registers a custom node with the `name`.
    ///
    /// `node_builder_fn` is the function that builds your node and returns it as a
    /// `Box<dyn NodeBase>`. This function will be called for every instance of
    /// that node defined in the XML behavior tree definition.
    ///
    /// # Converting your node into `Box<dyn NodeBase>`
    ///
    /// There's a helper trait with a blanket implementation that can be used to
    /// convert your custom node into a `Box<dyn NodeBase>`, [`ToBoxed`] with the
    /// method [`ToBoxed::to_boxed`]. See the examples for its usage.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// #[derive(Debug)]
    /// struct CustomNode {
    ///     foo: i32,
    ///     bar: String,
    /// }
    ///
    /// impl CustomNode {
    ///     // You can provide a custom new method
    ///     pub fn new(foo: i32) -> Self {
    ///         Self {
    ///             foo,
    ///             bar: String::new(),
    ///         }
    ///     }
    /// }
    ///
    /// impl SyncActionNode for CustomNode {
    ///     fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
    ///         Ok(NodeStatus::Success)
    ///     }
    /// }
    ///
    /// let mut registry = NodeRegistry::new();
    ///
    /// registry.insert(
    ///     "CustomNode",
    ///     // Here we call to_boxed() on the node after creation
    ///     || CustomNode::new(10).to_boxed(),
    ///     NodeType::Action
    /// );
    /// ```
    pub fn insert<F>(&mut self, name: impl AsRef<str>, node_builder_fn: F, node_type: NodeType)
    where
        F: Fn() -> Box<dyn NodeBase> + Send + Sync + 'static,
    {
        self.node_map
            .insert(name.as_ref().into(), (node_type, Arc::new(node_builder_fn)));
    }

    /// Merges another [`NodeRegistry`], where any duplicate entries will get
    /// replaced by values in `other`.
    pub fn merge(&mut self, other: NodeRegistry) {
        let mut other = other;

        self.node_map.extend(other.node_map.drain());
    }

    /// Find and build the node by name, if it matches a registered node
    pub(crate) fn build_node(&self, name: &str) -> Option<(NodeType, Box<dyn NodeBase>)> {
        let builtin = BUILTIN_NODES.get_or_init(builtin_nodes);

        // Try the built-in nodes first, then the user-provided nodes
        builtin
            .get(name)
            .or_else(|| self.node_map.get(name))
            .map(|(node_type, node_fn)| (*node_type, node_fn()))
    }
}

/// Registers all of the built-in nodes provided by `behaviortree_rs`
fn builtin_nodes() -> HashMap<String, (NodeType, Arc<NodeCreateFnDyn>)> {
    let mut node_map = HashMap::new();

    // Control nodes
    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::control::SequenceNode::default().to_boxed() })
            as Arc<NodeCreateFnDyn>;
    node_map.insert(String::from("Sequence"), (NodeType::Control, node));

    let node = Arc::new(|| -> Box<dyn NodeBase> {
        nodes::control::ReactiveSequenceNode::default().to_boxed()
    });
    node_map.insert(String::from("ReactiveSequence"), (NodeType::Control, node));

    let node = Arc::new(|| -> Box<dyn NodeBase> {
        nodes::control::SequenceWithMemoryNode::default().to_boxed()
    });
    node_map.insert(String::from("SequenceStar"), (NodeType::Control, node));

    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::control::ParallelNode::default().to_boxed() });
    node_map.insert(String::from("Parallel"), (NodeType::Control, node));

    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::control::ParallelAllNode::default().to_boxed() });
    node_map.insert(String::from("ParallelAll"), (NodeType::Control, node));

    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::control::FallbackNode::default().to_boxed() });
    node_map.insert(String::from("Fallback"), (NodeType::Control, node));

    let node = Arc::new(|| -> Box<dyn NodeBase> {
        nodes::control::ReactiveFallbackNode::default().to_boxed()
    });
    node_map.insert(String::from("ReactiveFallback"), (NodeType::Control, node));

    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::control::IfThenElseNode::default().to_boxed() });
    node_map.insert(String::from("IfThenElse"), (NodeType::Control, node));

    let node = Arc::new(|| -> Box<dyn NodeBase> { nodes::control::WhileDoElseNode.to_boxed() });
    node_map.insert(String::from("WhileDoElse"), (NodeType::Control, node));

    // Decorator nodes

    #[cfg(feature = "expr")]
    {
        // Condition node
        let node = Arc::new(|| -> Box<dyn NodeBase> {
            nodes::action::ConditionNode::default().to_boxed()
        });
        node_map.insert(String::from("Condition"), (NodeType::Action, node));
    }

    let node = Arc::new(|| -> Box<dyn NodeBase> { nodes::decorator::ForceFailureNode.to_boxed() });
    node_map.insert(String::from("ForceFailure"), (NodeType::Decorator, node));

    let node = Arc::new(|| -> Box<dyn NodeBase> { nodes::decorator::ForceSuccessNode.to_boxed() });
    node_map.insert(String::from("ForceSuccess"), (NodeType::Decorator, node));

    let node = Arc::new(|| -> Box<dyn NodeBase> { nodes::decorator::InverterNode.to_boxed() });
    node_map.insert(String::from("Inverter"), (NodeType::Decorator, node));

    let node = Arc::new(|| -> Box<dyn NodeBase> {
        nodes::decorator::KeepRunningUntilFailureNode.to_boxed()
    });
    node_map.insert(
        String::from("KeepRunningUntilFailure"),
        (NodeType::Decorator, node),
    );

    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::decorator::RepeatNode::default().to_boxed() });
    node_map.insert(String::from("Repeat"), (NodeType::Decorator, node));

    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::decorator::RetryNode::default().to_boxed() });
    node_map.insert(String::from("Retry"), (NodeType::Decorator, node));

    let node =
        Arc::new(|| -> Box<dyn NodeBase> { nodes::decorator::RunOnceNode::default().to_boxed() });
    node_map.insert(String::from("RunOnce"), (NodeType::Decorator, node));

    node_map
}
