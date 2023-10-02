/// Macro for simplifying implementation of `FromString` for any type that implements `FromStr`.
///
/// The macro-based implementation works for any type that implements `FromStr`;
/// it calls `parse()` under the hood.
#[doc(hidden)]
macro_rules! __impl_from_string {
    ( $($t:ty),* ) => {
        $(
            impl $crate::basic_types::FromString for $t
            {
                type Err = <$t as FromStr>::Err;

                fn from_string(value: impl AsRef<str>) -> Result<Self, Self::Err> {
                    value.as_ref().parse()
                }
            }
        ) *
    };
}
#[doc(inline)]
pub(crate) use __impl_from_string as impl_from_string;

/// Macro for simplifying implementation of `IntoString` for any type implementing `Display`.
///
/// Also implements the trait for `Vec<T>` for each type, creating a `;` delimited string,
/// calling `into_string()` on the item type.
///
/// Implementation works for any type that implements `Display`; it calls `to_string()`.
/// However, for custom implementations, don't include in this macro.
#[doc(hidden)]
macro_rules! __impl_into_string {
    ( $($t:ty),* ) => {
        $(
            impl $crate::basic_types::BTToString for $t {
                fn bt_to_string(&self) -> String {
                    self.to_string()
                }
            }

            impl $crate::basic_types::BTToString for Vec<$t> {
                fn bt_to_string(&self) -> String {
                    self
                    .iter()
                    .map(|x| x.bt_to_string())
                    .collect::<Vec<String>>()
                    .join(";")
            }
        }
    ) *
};
}
#[doc(inline)]
pub(crate) use __impl_into_string as impl_into_string;

#[macro_export]
#[doc(hidden)]
macro_rules! __define_ports {
    ( $($tu:expr),* ) => {
        {
            let mut ports = $crate::basic_types::PortsList::new();
            $(
                let (name, port_info) = $tu;
                ports.insert(String::from(name), port_info);
            )*

            ports
        }
    };
}
#[doc(inline)]
pub use __define_ports as define_ports;

#[macro_export]
#[doc(hidden)]
macro_rules! __input_port {
    ($n:tt) => {{
        use $crate::basic_types::{PortDirection, PortInfo};
        let port_info = PortInfo::new(PortDirection::Input);

        ($n, port_info)
    }};
    ($n:tt, $d:expr) => {{
        use $crate::basic_types::{PortDirection, PortInfo};
        let mut port_info = PortInfo::new(PortDirection::Input);

        port_info.set_default($d);

        ($n, port_info)
    }};
}
#[doc(inline)]
pub use __input_port as input_port;

#[macro_export]
#[doc(hidden)]
macro_rules! __output_port {
    ($n:tt) => {{
        use $crate::basic_types::{PortDirection, PortInfo};
        let port_info = PortInfo::new(PortDirection::Output);

        ($n, port_info)
    }};
}
#[doc(inline)]
pub use __output_port as output_port;

#[doc(hidden)]
macro_rules! __build_node_ptr {
    ($b:expr, $n:expr, $t:ty) => {
        {
            use $crate::nodes::{NodeConfig, GetNodeType, NodePorts, TreeNodeDefaults};

            let node_config = NodeConfig::new(::std::sync::Arc::clone(&$b));
            let mut node = <$t>::new($n, node_config);
            let manifest = $crate::basic_types::TreeNodeManifest::new(node.node_type(), $n, node.provided_ports(), "");
            node.config().set_manifest(::std::sync::Arc::new(manifest));
            let node = Box::new(node);
            node
        }
    };
    ($f:ident, $n:expr, $t:ty, $($x:expr),*) => {
        <$t>::new($n, $($x),*)
    };
}
#[doc(inline)]
pub(crate) use __build_node_ptr as build_node_ptr;

#[macro_export]
#[doc(hidden)]
macro_rules! __register_node {
    ($f:ident, $n:expr, $t:ty) => {
        {
            use $crate::nodes::{NodeConfig, GetNodeType, TreeNode, TreeNodeDefaults};
            use $crate::basic_types::{NodeType, TreeNodeManifest};
            use $crate::tree::NodePtrType;

            let blackboard = $f.blackboard();
            let node_config = NodeConfig::new(blackboard);
            let mut node = <$t>::new($n, node_config);
            let manifest = TreeNodeManifest {
                node_type: node.node_type(),
                registration_id: $n.to_string(),
                ports: node.provided_ports(),
                description: String::new(),
            };
            node.config().set_manifest(::std::rc::Rc::new(manifest));
            match node.node_type() {
                NodeType::Action => {
                    $f.register_node($n, NodePtrType::Action(Box::new(node)));
                }
                _ => panic!("Currently unsupported NodeType")
            };
        }
    };
    ($f:ident, $n:expr, $t:ty, $($x:expr),*) => {
        {
            use $crate::nodes::{NodeConfig, GetNodeType, TreeNode, TreeNodeDefaults};
            use $crate::basic_types::{NodeType, TreeNodeManifest};
            use $crate::tree::NodePtrType;

            let blackboard = $f.blackboard();
            let node_config = NodeConfig::new(blackboard);
            let mut node = <$t>::new($n, node_config, $($x),*);
            let manifest = TreeNodeManifest {
                node_type: node.node_type(),
                registration_id: $n.to_string(),
                ports: node.provided_ports(),
                description: String::new(),
            };
            node.config().set_manifest(::std::rc::Rc::new(manifest));
            match node.node_type() {
                NodeType::Action => {
                    $f.register_node($n, NodePtrType::Action(Box::new(node)));
                }
                _ => panic!("Currently unsupported NodeType")
            };
        }
    };
}
#[doc(inline)]
pub use __register_node as register_node;
