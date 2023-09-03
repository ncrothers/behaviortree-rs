
#[macro_export]
#[doc(hidden)]
macro_rules! __get_input {
    ($self:ident, $t:ident, $k:tt) => {
        {
            use $crate::blackboard::BlackboardString;
            use $crate::basic_types::StringInto;
            use std::any::TypeId;

            let value: Result<$t, $crate::nodes::NodeError> = match $self.config().input_ports.get($k) {
                Some(val) => {
                    // TODO: Check if default is needed
                    if val.is_empty() {
                        match $self.config().manifest() {
                            Ok(manifest) => {
                                let port_info = manifest.ports.get($k).unwrap();
                                match port_info.default_value() {
                                    Some(default) => {
                                        match default.bt_to_string().string_into() {
                                            Ok(value) => Ok(value),
                                            Err(e) => Err($crate::nodes::NodeError::PortError(String::from($k)))
                                        }
                                    },
                                    None => Err($crate::nodes::NodeError::PortError(String::from($k)))
                                }
                            }
                            Err(e) => {
                                Err($crate::nodes::NodeError::PortError(String::from($k)))
                            }
                        }
                    }
                    else {
                        match $crate::basic_types::get_remapped_key($k, val) {
                            Some(key) => {
                                match $self.config().blackboard.borrow().read::<$t>(&key) {
                                    Some(val) => Ok(val),
                                    None => Err($crate::nodes::NodeError::BlackboardError(key))
                                }
                            }
                            // Just a normal string
                            None => {
                                match val.string_into() {
                                    Ok(val) => Ok(val),
                                    Err(_) => Err($crate::nodes::NodeError::PortValueParseError(String::from($k), format!("{:?}", TypeId::of::<$t>())))
                                }
                            }
                        }
                    }
                }
                // Port not found
                None => Err($crate::nodes::NodeError::PortError(String::from($k)))
            };

            value
        }
    };
}
#[doc(inline)]
pub use __get_input as get_input;

#[macro_export]
#[doc(hidden)]
macro_rules! __set_output {
    ($k:tt, $v:expr) => {
        self.config.blackboard.borrow_mut().write($k, $v)
    };
}
#[doc(inline)]
pub use __set_output as set_output;

/// Macro for simplifying implementation of `StringInto<T>` for any type that implements `FromStr`.
///
/// Also implements the trait for `Vec<T>`, with a delimiter of `;`.
///
/// The macro-based implementation works for any type that implements `FromStr`; 
/// it calls `parse()` under the hood.
#[doc(hidden)]
macro_rules! __impl_string_into {
    ( $($t:ty),* ) => {
        $(
            impl<T> $crate::basic_types::StringInto<$t> for T
            where T: AsRef<str>
            {
                type Err = <$t as FromStr>::Err;

                fn string_into(&self) -> Result<$t, Self::Err> {
                    self.as_ref().parse()
                }
            }

            impl<T> $crate::basic_types::StringInto<Vec<$t>> for T
            where T: AsRef<str>
            {
                type Err = <$t as FromStr>::Err;

                fn string_into(&self) -> Result<Vec<$t>, Self::Err> {
                    self
                        .as_ref()
                        .split(";")
                        .map(|x| Ok(x.parse()?))
                        .collect()
                }
            }
        ) *
    };
}
#[doc(inline)]
pub(crate) use __impl_string_into as impl_string_into;

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
            let mut ports = PortsList::new();
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
    ($n:tt) => {
        {
            use $crate::basic_types::{PortInfo, PortDirection};
            let port_info = PortInfo::new(PortDirection::Input);
            
            ($n, port_info)
        }
    };
    ($n:tt, $d:expr) => {
        {
            use $crate::basic_types::{PortInfo, PortDirection};
            let mut port_info = PortInfo::new(PortDirection::Input);
            
            port_info.set_default($d);
            
            ($n, port_info)
        }
    };
}
#[doc(inline)]
pub use __input_port as input_port;

#[macro_export]
#[doc(hidden)]
macro_rules! __output_port {
    ($n:tt) => {
        {
            use $crate::basic_types::{PortInfo, PortDirection};
            let port_info = PortInfo::new(PortDirection::Output);
            
            ($n, port_info)
        }
    };
}
#[doc(inline)]
pub use __output_port as output_port;

#[macro_export]
#[doc(hidden)]
macro_rules! __register_node {
    ($f:ident, $n:expr, $t:ty) => {
        {
            use bt_cpp_rust::nodes::{NodeConfig, GetNodeType, TreeNode, TreeNodeDefaults};
            use bt_cpp_rust::basic_types::{NodeType, TreeNodeManifest};
            use bt_cpp_rust::tree::NodePtrType;
            
            let blackboard = $f.blackboard();
            let node_config = NodeConfig::new(blackboard);
            let mut node = <$t>::new($n, node_config);
            let manifest = TreeNodeManifest {
                node_type: node.node_type(),
                registration_id: $n.to_string(),
                ports: node.provided_ports(),
                description: String::new(),
            };
            node.config().set_manifest(Rc::new(manifest));
            match node.node_type() {
                NodeType::Action => {
                    $f.register_node($n, NodePtrType::Action(Box::new(node)));
                }
                _ => panic!("Currently unsupported NodeType")
            };
        }
    };
    ($f:ident, $n:expr, $t:ty, $($x:expr),*) => {
        <$t>::new($n, $($x),*)
    };
}
#[doc(inline)]
pub use __register_node as register_node;