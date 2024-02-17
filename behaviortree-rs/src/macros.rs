pub use behaviortree_rs_derive::{
    register_action_node, register_control_node, register_decorator_node,
};

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

#[macro_export]
#[doc(hidden)]
macro_rules! __build_node_ptr {
    ($conf:expr, $n:expr, $t:ty $(,$x:expr),* $(,)?) => {
        {
            let mut node = <$t>::create_node($n, $conf, $($x),*);
            let manifest = $crate::basic_types::TreeNodeManifest::new(node.node_type(), $n, node.provided_ports(), "");
            node.config_mut().set_manifest(::std::sync::Arc::new(manifest));
            // let node = Box::new(node);
            node
        }
    };
    // ($f:ident, $n:expr, $t:ty, $($x:expr),*) => {
    //     <$t>::new($n, $($x),*)
    // };
}
#[doc(inline)]
pub use __build_node_ptr as build_node_ptr;
