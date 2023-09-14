use std::{any::Any, collections::HashMap, rc::Rc, cell::RefCell};

use log::warn;

use crate::basic_types::{BTToString, StringInto, PortInfo};

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

/// Struct that stores arbitrary data in a `HashMap<String, Box<dyn Any>>`.
/// Data types must be compatible with `BTToString` and `StringInto<T>`.
/// 
/// Provides methods `get<T>()` and `set<T>()`.
/// 
/// # get
/// 
/// When reading from the Blackboard, a String will attempt to be coerced to
/// `T` by calling `string_into()`. `get<T>()` will return `None` if:
/// - No key matches the provided key
/// - The value type doesn't match the stored type (`.downcast<T>()`)
/// - Value is a string but `to_string()` returns `Err`
/// 
/// # set
/// 
/// `set()` returns an `Option<T>` which contains the previous value.
/// If there was no previous value, it returns `None`.
#[derive(Debug)]
pub struct Blackboard {
    storage: HashMap<String, EntryPtr>,
    parent_bb: Option<BlackboardPtr>,
    internal_to_external: HashMap<String, String>,
    auto_remapping: bool,
}

#[derive(Debug)]
pub struct Entry {
    pub value: Box<dyn Any>,
}

pub type BlackboardPtr = Rc<RefCell<Blackboard>>;
pub type EntryPtr = Rc<RefCell<Entry>>;

impl Blackboard {
    fn new(parent_bb: Option<BlackboardPtr>) -> Blackboard {
        Self {
            storage: HashMap::new(),
            parent_bb,
            internal_to_external: HashMap::new(),
            auto_remapping: false,
        }
    }

    /// Creates a Blackboard with `parent_bb` as the parent. Returned as a new `BlackboardPtr`.
    pub fn with_parent(parent_bb: &BlackboardPtr) -> BlackboardPtr {
        Rc::new(RefCell::new(Self::new(Some(Rc::clone(parent_bb)))))
    }

    /// Creates a Blackboard with no parent and returns it as a `BlackboardPtr`.
    pub fn new_ptr() -> BlackboardPtr {
        std::rc::Rc::new(std::cell::RefCell::new(Self::new(None)))
    }

    /// Enables the Blackboard to use autoremapping when getting values from 
    /// the parent Blackboard. Only uses autoremapping if there's no matching
    /// explicit remapping rule.
    pub fn enable_auto_remapping(&mut self, use_remapping: bool) {
        self.auto_remapping = use_remapping;
    }

    /// Adds remapping rule for Blackboard. Maps from `internal` (this Blackboard)
    /// to `external` (a parent Blackboard)
    pub fn add_subtree_remapping(&mut self, internal: String, external: String) {
        self.internal_to_external.insert(internal, external);
    }

    /// Get an Rc to the Entry
    fn get_entry(&mut self, key: impl AsRef<str>) -> Option<EntryPtr> {
        let key = key.as_ref().to_string();

        // Try to get the key
        if let Some(entry) = self.storage.get(&key) {
            return Some(Rc::clone(entry));
        }
        // Couldn't find key. Try remapping if we have a parent
        else if let Some(parent_bb) = self.parent_bb.as_ref() {
            if let Some(new_key) = self.internal_to_external.get(&key) {
                // Return the value of the parent's `get()`
                let parent_entry = parent_bb.borrow_mut().get_entry(new_key);

                if let Some(value) = &parent_entry {
                    self.storage.insert(key, Rc::clone(value));
                }

                return parent_entry;
            }
            // Use auto remapping
            else if self.auto_remapping {
                // Return the value of the parent's `get()`
                return parent_bb.borrow_mut().get_entry(key);
            }
        }

        // No matches
        None
    }

    /// The `Blackboard` tries a few things when reading a `key`:
    /// - First it checks if it can find `key`:
    ///     - Check itself for `key`
    ///     - If it doesn't exist, if`self` has a parent `Blackboard`, it checks for key remapping
    ///         - If a remapping rule exists for `key`, use the remapped `key`
    ///         - If `auto_remapping` is enabled, it uses `key` directly
    ///     - Return `None` if none of the above work
    /// - If a value is matched, attempt to coerce the value to `T`. If it couldn't 
    /// be coerced to `T`:
    ///     - If it's a `String` or `&str`, try calling `string_into()`
    /// - If none of those work, return `None`
    /// 
    /// __NOTE__: This method borrows `self` mutably because if it finds a remapped
    /// key from the parent `Blackboard`, it stores the `EntryPtr` in `self` so
    /// the next lookup for it is quicker.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use bt_cpp_rust::blackboard::Blackboard;
    /// 
    /// let mut blackboard = Blackboard::new_ptr();
    /// let mut blackboard_borrowed = blackboard.borrow_mut();
    /// 
    /// blackboard_borrowed.set("foo", 132u32);
    /// assert_eq!(blackboard_borrowed.get::<u32>("foo"), Some(132u32));
    /// 
    /// blackboard_borrowed.set("bar", "100");
    /// 
    /// assert_eq!(blackboard_borrowed.get::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard_borrowed.get::<u32>("bar"), Some(100u32));
    /// ```
    pub fn get<T>(&mut self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone,
        String: StringInto<T>,
    {
        let key = key.as_ref().to_string();

        // Try to get the key
        if let Some(entry) = self.get_entry(&key) {
            // Try to downcast directly to T
            if let Some(value) = entry.borrow().value.downcast_ref::<T>() {
                return Some(value.clone());
            } else {
                // If value is a String or &str, try to call `StringInto` to convert to T
                if let Some(value) = entry.borrow().value.downcast_ref::<String>() {
                    if let Ok(value) = value.string_into() {
                        return Some(value);
                    }
                } else if let Some(value) = entry.borrow().value.downcast_ref::<&str>() {
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

    /// Sets the `value` in the Blackboard at `key`.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use bt_cpp_rust::blackboard::Blackboard;
    /// 
    /// let mut blackboard = Blackboard::new_ptr();
    /// let mut blackboard_borrowed = blackboard.borrow_mut();
    /// 
    /// blackboard_borrowed.set("foo", 132u32);
    /// assert_eq!(blackboard_borrowed.get::<u32>("foo"), Some(132u32));
    /// 
    /// blackboard_borrowed.set("bar", "100");
    /// 
    /// assert_eq!(blackboard_borrowed.get::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard_borrowed.get::<u32>("bar"), Some(100u32));
    /// ```
    pub fn set<T: Any + BTToString + 'static>(
        &mut self,
        key: impl AsRef<str>,
        value: T,
    ) {
        let key = key.as_ref().to_string();

        if let Some(entry) = self.storage.get_mut(&key) {
            entry.borrow_mut().value = Box::new(value);
        }
        else {
            let entry = Entry { value: Box::new(value) };
    
            self.storage.insert(key, Rc::new(RefCell::new(entry)));
        }
    }
}
