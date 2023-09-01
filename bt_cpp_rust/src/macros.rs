
macro_rules! get_input {
    ($self:ident, $t:ident, $k:tt) => {
        $self.config.blackboard.borrow().read::<$t>($k)
    };
}
pub(crate) use get_input;

macro_rules! set_output {
    ($k:tt, $v:expr) => {
        self.config.blackboard.borrow_mut().write($k, $v)
    };
}
pub(crate) use set_output;

/// Macro for simplifying implementation of `StringInto<T>` for any type that implements `FromStr`.
///
/// Also implements the trait for `Vec<T>`, with a delimiter of `;`.
///
/// The macro-based implementation works for any type that implements `FromStr`; 
/// it calls `parse()` under the hood.
macro_rules! impl_string_into {
    ( $($t:ty),* ) => {
        $(
            impl<T> StringInto<$t> for T
            where T: AsRef<str>
            {
                type Err = <$t as FromStr>::Err;

                fn string_into(&self) -> Result<$t, Self::Err> {
                    self.as_ref().parse()
                }
            }

            impl<T> StringInto<Vec<$t>> for T
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
pub(crate) use impl_string_into;

/// Macro for simplifying implementation of `IntoString` for any type implementing `Display`.
///
/// Also implements the trait for `Vec<T>` for each type, creating a `;` delimited string,
/// calling `into_string()` on the item type.
///
/// Implementation works for any type that implements `Display`; it calls `to_string()`.
/// However, for custom implementations, don't include in this macro.
macro_rules! impl_into_string {
    ( $($t:ty),* ) => {
        $(
            impl BTToString for $t {
                fn bt_to_string(&self) -> String {
                    self.to_string()
                }
            }

            impl BTToString for Vec<$t> {
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
pub(crate) use impl_into_string;

macro_rules! define_ports {
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
pub(crate) use define_ports;

macro_rules! input_port {
    ($n:tt) => {
        {
            use crate::basic_types::{PortInfo, PortDirection};
            let port_info = PortInfo::new(PortDirection::Input);
    
            ($n, port_info)
        }
    };
}
pub(crate) use input_port;

macro_rules! output_port {
    ($n:tt) => {
        {
            use crate::basic_types::{PortInfo, PortDirection};
            let port_info = PortInfo::new(PortDirection::Output);
    
            ($n, port_info)
        }
    };
}
pub(crate) use output_port;
