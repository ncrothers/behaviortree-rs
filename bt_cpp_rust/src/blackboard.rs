use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use futures::future::BoxFuture;
use tokio::sync::{Mutex, RwLock};

use crate::basic_types::{FromString, ParseStr};

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
///
/// # Usage
///
/// Create a root-level `Blackboard` using `Blackboard::create()`, which returns
/// a `BlackboardPtr`.
///
/// ```
/// # tokio_test::block_on(async {
/// use bt_cpp_rust::Blackboard;
///
/// // Create a root-level Blackboard
/// let bb = Blackboard::create();
/// // Create a child Blackboard
/// let child = Blackboard::with_parent(&bb).await;
/// # })
/// ```
///
/// Provides methods `get<T>()`, `get_exact<T>()`, and `set<T>()`.
///
/// ## get
///
/// When reading from the Blackboard, a String will attempt to be coerced to
/// `T` by calling `parse_str()`. `get<T>()` will return `None` if:
/// - No key matches the provided key
/// - The value type doesn't match the stored type (`.downcast<T>()`)
/// - Value is a string but `to_string()` returns `Err`
///
/// ## get_exact
///
/// If the value type at the key doesn't match `T`, it will _not_ try to
/// parse a string value. It will just return `None`.
#[derive(Debug, Clone)]
pub struct Blackboard {
    data: Arc<RwLock<BlackboardData>>,
    parent_bb: Box<Option<Blackboard>>,
}

#[derive(Debug)]
pub struct BlackboardData {
    storage: HashMap<String, EntryPtr>,
    internal_to_external: HashMap<String, String>,
    auto_remapping: bool,
}

#[derive(Debug)]
pub struct Entry {
    pub value: Box<dyn Any + Send>,
}

pub type BlackboardPtr = Arc<RwLock<Blackboard>>;
pub type BlackboardDataPtr = Arc<RwLock<BlackboardData>>;

pub type EntryPtr = Arc<Mutex<Entry>>;

impl Blackboard {
    fn new(parent_bb: Option<Blackboard>) -> Blackboard {
        Self {
            data: Arc::new(RwLock::new(BlackboardData {
                storage: HashMap::new(),
                internal_to_external: HashMap::new(),
                auto_remapping: false,
            })),
            parent_bb: Box::new(parent_bb),
        }
    }

    fn data_ptr(&self) -> BlackboardDataPtr {
        Arc::clone(&self.data)
    }

    fn parent(&self) -> Option<Blackboard> {
        self.parent_bb.as_ref().as_ref().cloned()
    }

    /// Creates a Blackboard with `parent_bb` as the parent. Returned as a new `BlackboardPtr`.
    pub async fn with_parent(parent_bb: &Blackboard) -> Blackboard {
        Self::new(Some(parent_bb.clone()))
    }

    /// Sync version of `with_parent()`
    ///
    /// Creates a Blackboard with `parent_bb` as the parent. Returned as a new `BlackboardPtr`.
    pub fn with_parent_sync(parent_bb: &Blackboard) -> Blackboard {
        crate::sync::block_on(Self::with_parent(parent_bb))
    }

    /// Creates a Blackboard with no parent and returns it as a `BlackboardPtr`.
    pub fn create() -> Blackboard {
        Self {
            parent_bb: Box::new(None),
            data: Arc::new(RwLock::new(BlackboardData {
                storage: HashMap::new(),
                internal_to_external: HashMap::new(),
                auto_remapping: false,
            })),
        }
    }

    /// Enables the Blackboard to use autoremapping when getting values from
    /// the parent Blackboard. Only uses autoremapping if there's no matching
    /// explicit remapping rule.
    pub async fn enable_auto_remapping(&mut self, use_remapping: bool) {
        self.data.write().await.auto_remapping = use_remapping;
    }

    /// Sync version of `enable_auto_remapping()`
    ///
    /// Enables the Blackboard to use autoremapping when getting values from
    /// the parent Blackboard. Only uses autoremapping if there's no matching
    /// explicit remapping rule.
    pub fn enable_auto_remapping_sync(&mut self, use_remapping: bool) {
        crate::sync::block_on(self.enable_auto_remapping(use_remapping))
    }

    /// Adds remapping rule for Blackboard. Maps from `internal` (this Blackboard)
    /// to `external` (a parent Blackboard)
    pub async fn add_subtree_remapping(&mut self, internal: String, external: String) {
        self.data
            .write()
            .await
            .internal_to_external
            .insert(internal, external);
    }

    /// Sync version of add_subtree_remapping
    ///
    /// Adds remapping rule for Blackboard. Maps from `internal` (this Blackboard)
    /// to `external` (a parent Blackboard)
    pub fn add_subtree_remapping_sync(&mut self, internal: String, external: String) {
        crate::sync::block_on(self.add_subtree_remapping(internal, external));
    }

    /// Get an Rc to the Entry
    fn get_entry<'a>(&'a mut self, key: &'a str) -> BoxFuture<Option<EntryPtr>> {
        Box::pin(async move {
            let mut blackboard = self.data.write().await;

            // Try to get the key
            if let Some(entry) = blackboard.storage.get(key) {
                return Some(Arc::clone(entry));
            }
            // Couldn't find key. Try remapping if we have a parent
            else if let Some(parent_bb) = self.parent_bb.as_mut() {
                if let Some(new_key) = blackboard.internal_to_external.get(key) {
                    // Return the value of the parent's `get()`
                    let parent_entry = parent_bb.get_entry(new_key).await;

                    if let Some(value) = &parent_entry {
                        blackboard
                            .storage
                            .insert(key.to_string(), Arc::clone(value));
                    }

                    return parent_entry;
                }
                // Use auto remapping
                else if blackboard.auto_remapping {
                    // Return the value of the parent's `get()`
                    return parent_bb.get_entry(key).await;
                }
            }

            // No matches
            None
        })
    }

    /// Internal method that just tries to get value at key. If the stored
    /// type is not T, return None
    async fn __get_no_string<T>(&mut self, key: &str) -> Option<T>
    where
        T: Any + Clone,
    {
        // Try to get the key
        if let Some(entry) = self.get_entry(key).await {
            // Try to downcast directly to T
            if let Some(value) = entry.lock().await.value.downcast_ref::<T>() {
                return Some(value.clone());
            }
        }

        None
    }

    /// Internal method that tries to get the value at key, but only works
    /// if it's a String/&str, then tries FromString to convert it to T
    async fn __get_allow_string<T>(&mut self, key: &str) -> Option<T>
    where
        T: Any + Clone + FromString + Send,
    {
        // Try to get the key
        if let Some(entry) = self.get_entry(key).await {
            let value = {
                let entry_lock = entry.lock().await;
                // If value is a String or &str, try to call `FromString` to convert to T
                if let Some(value) = entry_lock.value.downcast_ref::<String>() {
                    value.to_string()
                } else if let Some(value) = entry_lock.value.downcast_ref::<&str>() {
                    value.to_string()
                }
                // Didn't match either String or &str, so return from __get_allow_string
                else {
                    return None;
                }
            };

            // Try to parse String into T
            if let Ok(value) = <String as ParseStr<T>>::parse_str(&value) {
                // Update value with the value type instead of just a string
                let mut t = entry.lock().await;
                t.value = Box::new(value.clone());
                return Some(value);
            }
        }

        // No matches
        None
    }

    /// Tries to return the value at `key`. The type `T` must implement
    /// `FromString` when calling this method; it will try to convert
    /// from `String`/`&str` if there's an entry at `key` but it is not
    /// of type `T`. If it does convert it successfully, it will replace
    /// the existing value with `T` so converting from the string type
    /// won't be needed next time.
    ///
    /// If you want to get an entry that has a type that doesn't implement
    /// `FromString`, use `get_exact<T>` instead.
    ///
    /// The `Blackboard` tries a few things when reading a `key`:
    /// - First it checks if it can find `key`:
    ///     - Check itself for `key`
    ///     - If it doesn't exist, if`self` has a parent `Blackboard`, it checks for key remapping
    ///         - If a remapping rule exists for `key`, use the remapped `key`
    ///         - If `auto_remapping` is enabled, it uses `key` directly
    ///     - Return `None` if none of the above work
    /// - If a value is matched, attempt to coerce the value to `T`. If it couldn't
    /// be coerced to `T`:
    ///     - If it's a `String` or `&str`, try calling `parse_str()`
    /// - If none of those work, return `None`
    ///
    /// __NOTE__: This method borrows `self` mutably because if it finds a remapped
    /// key from the parent `Blackboard`, it stores the `EntryPtr` in `self` so
    /// the next lookup for it is quicker.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use bt_cpp_rust::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("foo", 132u32).await;
    /// assert_eq!(blackboard.get::<u32>("foo").await, Some(132u32));
    ///
    /// blackboard.set("bar", "100").await;
    ///
    /// assert_eq!(blackboard.get::<String>("bar").await, Some(String::from("100")));
    /// assert_eq!(blackboard.get::<u32>("bar").await, Some(100u32));
    /// # })
    /// ```
    pub async fn get<T>(&mut self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone + FromString + Send,
    {
        // Try without parsing string first, then try with parsing string
        self.__get_no_string(key.as_ref())
            .await
            .or(self.__get_allow_string(key.as_ref()).await)
    }

    /// Sync version of `get<T>`
    ///
    /// Tries to return the value at `key`. The type `T` must implement
    /// `FromString` when calling this method; it will try to convert
    /// from `String`/`&str` if there's an entry at `key` but it is not
    /// of type `T`. If it does convert it successfully, it will replace
    /// the existing value with `T` so converting from the string type
    /// won't be needed next time.
    ///
    /// If you want to get an entry that has a type that doesn't implement
    /// `FromString`, use `get_exact<T>` instead.
    ///
    /// The `Blackboard` tries a few things when reading a `key`:
    /// - First it checks if it can find `key`:
    ///     - Check itself for `key`
    ///     - If it doesn't exist, if`self` has a parent `Blackboard`, it checks for key remapping
    ///         - If a remapping rule exists for `key`, use the remapped `key`
    ///         - If `auto_remapping` is enabled, it uses `key` directly
    ///     - Return `None` if none of the above work
    /// - If a value is matched, attempt to coerce the value to `T`. If it couldn't
    /// be coerced to `T`:
    ///     - If it's a `String` or `&str`, try calling `parse_str()`
    /// - If none of those work, return `None`
    ///
    /// __NOTE__: This method borrows `self` mutably because if it finds a remapped
    /// key from the parent `Blackboard`, it stores the `EntryPtr` in `self` so
    /// the next lookup for it is quicker.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use bt_cpp_rust::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("foo", 132u32).await;
    /// assert_eq!(blackboard.get::<u32>("foo").await, Some(132u32));
    ///
    /// blackboard.set("bar", "100").await;
    ///
    /// assert_eq!(blackboard.get::<String>("bar").await, Some(String::from("100")));
    /// assert_eq!(blackboard.get::<u32>("bar").await, Some(100u32));
    /// # })
    /// ```
    pub fn get_sync<T>(&mut self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone + FromString + Send,
    {
        futures::executor::block_on(self.get(key))
    }

    /// Version of `get<T>` that does _not_ try to convert from string if the type
    /// doesn't match. This method has the benefit of not requiring the trait
    /// `FromString`, which allows you to avoid implementing the trait for
    /// types that don't need it or it's impossible to represent the data
    /// type as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use bt_cpp_rust::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("foo", 132u32).await;
    /// assert_eq!(blackboard.get_exact::<u32>("foo").await, Some(132u32));
    ///
    /// blackboard.set("bar", "100").await;
    ///
    /// assert_eq!(blackboard.get_exact::<&str>("bar").await, Some("100"));
    /// assert_eq!(blackboard.get_exact::<String>("bar").await, None);
    /// assert_eq!(blackboard.get_exact::<u32>("bar").await, None);
    /// # })
    /// ```
    pub async fn get_exact<T>(&mut self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone,
    {
        self.__get_no_string(key.as_ref()).await
    }

    /// Sync version of `get_exact<T>`
    ///
    /// Version of `get<T>` that does _not_ try to convert from string if the type
    /// doesn't match. This method has the benefit of not requiring the trait
    /// `FromString`, which allows you to avoid implementing the trait for
    /// types that don't need it or it's impossible to represent the data
    /// type as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use bt_cpp_rust::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set_sync("foo", 132u32);
    /// assert_eq!(blackboard.get_exact_sync::<u32>("foo"), Some(132u32));
    ///
    /// blackboard.set_sync("bar", "100");
    ///
    /// assert_eq!(blackboard.get_exact_sync::<&str>("bar"), Some("100"));
    /// assert_eq!(blackboard.get_exact_sync::<String>("bar"), None);
    /// assert_eq!(blackboard.get_exact_sync::<u32>("bar"), None);
    /// # })
    /// ```
    pub fn get_exact_sync<T>(&mut self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone,
    {
        futures::executor::block_on(self.get_exact(key))
    }

    /// Sets the `value` in the Blackboard at `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use bt_cpp_rust::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("foo", 132u32).await;
    /// assert_eq!(blackboard.get::<u32>("foo").await, Some(132u32));
    ///
    /// blackboard.set("bar", "100").await;
    ///
    /// assert_eq!(blackboard.get::<String>("bar").await, Some(String::from("100")));
    /// assert_eq!(blackboard.get::<u32>("bar").await, Some(100u32));
    /// # })
    /// ```
    pub async fn set<T: Any + Send + 'static>(&mut self, key: impl AsRef<str>, value: T) {
        let key = key.as_ref().to_string();

        let mut blackboard = self.data.write().await;

        if let Some(entry) = blackboard.storage.get_mut(&key) {
            entry.lock().await.value = Box::new(value);
        } else {
            drop(blackboard);
            let entry = self.create_entry(&key).await;

            // Set value of new entry
            entry.lock().await.value = Box::new(value);
        }
    }

    /// Sync version of `set<T>`
    ///
    /// Sets the `value` in the Blackboard at `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// # tokio_test::block_on(async {
    /// use bt_cpp_rust::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set_sync("foo", 132u32);
    /// assert_eq!(blackboard.get_sync::<u32>("foo"), Some(132u32));
    ///
    /// blackboard.set_sync("bar", "100");
    ///
    /// assert_eq!(blackboard.get_sync::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard.get_sync::<u32>("bar"), Some(100u32));
    /// # })
    /// ```
    pub fn set_sync<T: Any + Send + 'static>(&mut self, key: impl AsRef<str>, value: T) {
        futures::executor::block_on(self.set(key, value))
    }

    fn create_entry<'a>(&'a mut self, key: &'a (impl AsRef<str> + Sync)) -> BoxFuture<EntryPtr> {
        Box::pin(async move {
            let entry;

            let mut blackboard = self.data.write().await;

            // If the entry already exists
            if let Some(existing_entry) = blackboard.storage.get(key.as_ref()) {
                return Arc::clone(existing_entry);
            }
            // Use explicit remapping rule
            else if blackboard.internal_to_external.contains_key(key.as_ref())
                && self.parent_bb.is_some()
            {
                // Safe to unwrap because .contains_key() is true
                let remapped_key = blackboard.internal_to_external.get(key.as_ref()).unwrap();

                entry = (*self.parent_bb)
                    .as_mut()
                    .unwrap()
                    .create_entry(remapped_key)
                    .await;
            }
            // Use autoremapping
            else if blackboard.auto_remapping && self.parent_bb.is_some() {
                entry = (*self.parent_bb).as_mut().unwrap().create_entry(key).await;
            }
            // No remapping or no parent blackboard
            else {
                // Create an entry with an empty placeholder value
                entry = Arc::new(Mutex::new(Entry {
                    value: Box::new(()),
                }));
            }

            blackboard
                .storage
                .insert(key.as_ref().to_string(), Arc::clone(&entry));
            entry
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: add other tests

    #[tokio::test]
    async fn create_entry() {
        // With no remapping

        let mut root_bb = Blackboard::create();
        let mut left_bb = Blackboard::with_parent(&root_bb).await;
        let mut right_bb = Blackboard::with_parent(&root_bb).await;

        left_bb.set("foo", 123u32).await;

        assert!(left_bb.get::<u32>("foo").await.is_some());
        // These two should be none because remapping is not enabled
        assert!(right_bb.get::<u32>("foo").await.is_none());
        assert!(root_bb.get::<u32>("foo").await.is_none());

        // With autoremapping

        let mut root_bb = Blackboard::create();
        let mut left_bb = Blackboard::with_parent(&root_bb).await;
        let mut right_bb = Blackboard::with_parent(&root_bb).await;

        root_bb.enable_auto_remapping(true).await;
        left_bb.enable_auto_remapping(true).await;
        right_bb.enable_auto_remapping(true).await;

        left_bb.set("foo", 123u32).await;

        assert_eq!(left_bb.get::<u32>("foo").await, Some(123));
        assert_eq!(right_bb.get::<u32>("foo").await, Some(123));
        assert_eq!(root_bb.get::<u32>("foo").await, Some(123));

        // With custom remapping
        let mut root_bb = Blackboard::create();
        let mut left_bb = Blackboard::with_parent(&root_bb).await;
        let mut right_bb = Blackboard::with_parent(&root_bb).await;

        right_bb
            .add_subtree_remapping(String::from("foo"), String::from("bar"))
            .await;
        left_bb
            .add_subtree_remapping(String::from("foo"), String::from("bar"))
            .await;

        left_bb.set("foo", 123u32).await;

        assert_eq!(left_bb.get::<u32>("foo").await, Some(123));
        assert_eq!(right_bb.get::<u32>("foo").await, Some(123));
        assert_eq!(root_bb.get::<u32>("bar").await, Some(123));
    }

    #[tokio::test]
    async fn remapping() {
        // No remapping

        let mut root_bb = Blackboard::create();
        let mut child_bb = Blackboard::with_parent(&root_bb).await;

        root_bb.set("foo", 123u32).await;

        assert!(child_bb.get::<u32>("foo").await.is_none());

        // Auto remapping

        let mut root_bb = Blackboard::create();
        let mut child1_bb = Blackboard::with_parent(&root_bb).await;
        let mut child2_bb = Blackboard::with_parent(&child1_bb).await;
        let mut child3_bb = Blackboard::with_parent(&child2_bb).await;

        child1_bb.enable_auto_remapping(true).await;
        child2_bb.enable_auto_remapping(true).await;
        child3_bb.enable_auto_remapping(true).await;

        root_bb.set("foo", 123u32).await;

        assert_eq!(child1_bb.get::<u32>("foo").await, Some(123));
        assert_eq!(child2_bb.get::<u32>("foo").await, Some(123));
        assert_eq!(child3_bb.get::<u32>("foo").await, Some(123));

        // Custom remapping

        let mut root_bb = Blackboard::create();
        let mut child1_bb = Blackboard::with_parent(&root_bb).await;
        let mut child2_bb = Blackboard::with_parent(&child1_bb).await;
        let mut child3_bb = Blackboard::with_parent(&child2_bb).await;

        child1_bb
            .add_subtree_remapping(String::from("child1"), String::from("root"))
            .await;
        child2_bb
            .add_subtree_remapping(String::from("child2"), String::from("child1"))
            .await;
        child3_bb
            .add_subtree_remapping(String::from("child3"), String::from("child2"))
            .await;

        root_bb.set("root", 123u32).await;

        assert_eq!(child1_bb.get::<u32>("child1").await, Some(123));
        assert_eq!(child2_bb.get::<u32>("child2").await, Some(123));
        assert_eq!(child3_bb.get::<u32>("child3").await, Some(123));
        assert_eq!(child3_bb.get::<u32>("foo").await, None);
    }

    #[tokio::test]
    async fn type_matching() {
        let mut bb = Blackboard::create();

        bb.set("foo", 123u32).await;

        assert!(bb.get::<u32>("foo").await.is_some());
        assert!(bb.get::<String>("foo").await.is_none());
        assert!(bb.get::<f32>("foo").await.is_none());
    }

    #[tokio::test]
    async fn custom_type() {
        #[derive(Clone, Debug, PartialEq)]
        struct CustomEntry {
            pub foo: u32,
            pub bar: String,
        }

        impl FromString for CustomEntry {
            type Err = anyhow::Error;

            fn from_string(value: impl AsRef<str>) -> Result<Self, Self::Err> {
                let splits: Vec<&str> = value.as_ref().split(',').collect();

                if splits.len() != 2 {
                    Err(anyhow::anyhow!("Error!"))
                } else {
                    let foo = splits[0].parse()?;
                    Ok(CustomEntry {
                        foo,
                        bar: splits[1].to_string(),
                    })
                }
            }
        }

        let mut bb = Blackboard::create();

        let custom_value = CustomEntry {
            foo: 123,
            bar: String::from("bar"),
        };

        bb.set("custom", custom_value.clone()).await;
        bb.set("custom_str", String::from("123,bar")).await;
        bb.set("custom_str_malformed", String::from("not an int,bar"))
            .await;

        assert_eq!(
            bb.get_exact::<CustomEntry>("custom").await.as_ref(),
            Some(&custom_value)
        );
        // Check parse from String
        assert_eq!(
            bb.get::<CustomEntry>("custom_str").await.as_ref(),
            Some(&custom_value)
        );
        // Check it returns None if it cannot be parsed
        assert_eq!(
            bb.get::<CustomEntry>("custom_str_malformed").await.as_ref(),
            None
        );
    }
}
