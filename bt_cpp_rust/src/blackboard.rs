use std::{any::Any, collections::HashMap, cell::RefCell};

use crate::basic_types::{BTToString, StringInto};

/// Trait that provides `strip_bb_pointer()` for all `AsRef<str>`,
/// which includes `String` and `&str`. 
pub trait BlackboardString {
    /// If not a blackboard pointer (i.e. `"value"`, instead of `"{value}"`), return
    /// `None`. If a blackboard pointer, remove brackets.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use bt_cpp_rust::blackboard::BlackboardString;
    /// 
    /// assert_eq!("value".strip_bb_pointer(), None);
    /// 
    /// assert_eq!("{value}".strip_bb_pointer(), Some(String::from("value")));
    /// ```
    fn strip_bb_pointer(&self) -> Option<String>;
    fn is_bb_pointer(&self) -> bool;
}

impl<T> BlackboardString for T
where
    T: AsRef<str> + Clone,
{
    fn strip_bb_pointer(&self) -> Option<String> {
        let str_ref = self.as_ref();

        // Is bb pointer
        if str_ref.starts_with('{') && str_ref.ends_with('}') {
            Some(
                str_ref
                    .strip_prefix('{')
                    .unwrap()
                    .strip_suffix('}')
                    .unwrap()
                    .to_string(),
            )
        } else {
            None
        }
    }

    fn is_bb_pointer(&self) -> bool {
        let str_ref = self.as_ref();
        str_ref.starts_with('{') && str_ref.ends_with('}')
    }
}

#[derive(Debug)]
/// Struct that stores arbitrary data in a `HashMap<String, Box<dyn Any>>`.
/// Data types must be compatible with `BTToString` and `StringInto<T>`.
/// 
/// Provides methods `read<T>()` and `write<T>()`.
/// 
/// # Read
/// 
/// When reading from the Blackboard, a String will attempt to be coerced to
/// `T` by calling `string_into()`. `read<T>()` will return `None` if:
/// - No key matches the provided key
/// - The value type doesn't match the stored type (`.downcast<T>()`)
/// - Value is a string but `to_string()` returns `Err`
/// 
/// # Write
/// 
/// `write()` returns an `Option<T>` which contains the previous value.
/// If there was no previous value, it returns `None`.
pub struct Blackboard {
    map: HashMap<String, Box<dyn Any>>,
}

pub type BlackboardPtr = std::rc::Rc<std::cell::RefCell<Blackboard>>;

impl Blackboard {
    pub fn new() -> Blackboard {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn new_ptr() -> BlackboardPtr {
        std::rc::Rc::new(std::cell::RefCell::new(Self::new()))
    }

    /// When reading from the Blackboard, a String will attempt to be coerced to
    /// `T` by calling `string_into()`. `read<T>()` will return `None` if:
    /// - No key matches the provided key
    /// - The value type doesn't match the stored type (`.downcast<T>()`)
    /// - Value is a string but `to_string()` returns `Err`
    /// 
    /// # Examples
    /// 
    /// ```
    /// use bt_cpp_rust::blackboard::Blackboard;
    /// 
    /// let mut blackboard = Blackboard::new();
    /// 
    /// assert_eq!(blackboard.write("foo", 132u32), None);
    /// assert_eq!(blackboard.read::<u32>("foo"), Some(132u32));
    /// 
    /// assert_eq!(blackboard.write("foo", 0u32), Some(132u32));
    /// 
    /// blackboard.write("bar", "100");
    /// 
    /// assert_eq!(blackboard.read::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard.read::<u32>("bar"), Some(100u32));
    /// ```
    pub fn read<T>(&self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone,
        String: StringInto<T>,
    {
        let key = key.as_ref().to_string();

        // Try to get the key
        if let Some(value) = self.map.get(&key) {
            // Try to downcast directly to T
            if let Some(value) = value.downcast_ref::<T>() {
                return Some(value.clone());
            } else {
                // If value is a String or &str, try to call `StringInto` to convert to T
                if let Some(value) = value.downcast_ref::<String>() {
                    if let Ok(value) = value.string_into() {
                        return Some(value);
                    }
                } else if let Some(value) = value.downcast_ref::<&str>() {
                    let value = value.to_string();
                    if let Ok(value) = value.string_into() {
                        return Some(value);
                    }
                }
            }
        }

        // No matches
        None
    }

    /// `write()` returns an `Option<T>` which contains the previous value.
    /// If there was no previous value, it returns `None`.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use bt_cpp_rust::blackboard::Blackboard;
    /// 
    /// let mut blackboard = Blackboard::new();
    /// 
    /// assert_eq!(blackboard.write("foo", 132u32), None);
    /// assert_eq!(blackboard.read::<u32>("foo"), Some(132u32));
    /// 
    /// assert_eq!(blackboard.write("foo", 0u32), Some(132u32));
    /// 
    /// blackboard.write("bar", "100");
    /// 
    /// assert_eq!(blackboard.read::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard.read::<u32>("bar"), Some(100u32));
    /// ```
    pub fn write<T: Any + BTToString + 'static>(
        &mut self,
        key: impl AsRef<str>,
        value: T,
    ) -> Option<T> {
        let prev = self.map.insert(key.as_ref().to_string(), Box::new(value));

        match prev {
            Some(prev) => match prev.downcast::<T>() {
                Ok(prev) => Some(*prev),
                Err(_) => None,
            },
            None => None,
        }
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}