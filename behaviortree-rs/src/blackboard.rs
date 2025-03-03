use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use ouroboros::self_referencing;
use parking_lot::{Mutex, MutexGuard, RwLock};

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
    /// use behaviortree_rs::blackboard::BlackboardString;
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

/// Struct that stores arbitrary data in a `HashMap<String, Box<dyn Any + Send>>`. Note the
/// stored data type _must_ implement `Send`.
///
/// # Usage
///
/// Create a root-level `Blackboard` using `Blackboard::create()`, which returns
/// a `BlackboardPtr`.
///
/// ```
/// use behaviortree_rs::Blackboard;
///
/// // Create a root-level Blackboard
/// let bb = Blackboard::create();
/// // Create a child Blackboard
/// let child = Blackboard::with_parent(&bb);
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

#[derive(Debug, Default)]
pub struct BlackboardData {
    storage: HashMap<String, EntryPtr>,
    internal_to_external: HashMap<String, String>,
    auto_remapping: bool,
}

#[derive(Debug)]
pub struct Entry(pub Box<dyn Any + Send>);

impl Deref for Entry {
    type Target = Box<dyn Any + Send>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Entry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[self_referencing]
pub struct EntryInner<T>
where
    T: 'static,
{
    entry: EntryPtr,
    #[borrows(mut entry)]
    #[covariant]
    guard: MutexGuard<'this, Entry>,
    #[borrows(guard)]
    value: &'this T,
}

pub struct EntryGuard<T: 'static>(EntryInner<T>);

impl<T> EntryGuard<T>
where
    T: 'static,
{
    fn create(entry: EntryPtr) -> Option<Self> {
        // Check if the inner value can be downcasted directly to `T`
        let is_valid = entry.lock().downcast_ref::<T>().is_some();

        if is_valid {
            let inner = EntryInner::new(
                entry,
                |entry| entry.lock(),
                |guard| {
                    guard
                        .downcast_ref::<T>()
                        .expect("downcasting should always be safe here")
                },
            );

            Some(Self(inner))
        } else {
            None
        }
    }

    /// Clones the type `T` and consumes `self`, which will release the lock
    pub fn clone_consume(self) -> T
    where
        T: Clone,
    {
        self.deref().clone()
    }
}

impl<T> Deref for EntryGuard<T>
where
    T: 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.borrow_value()
    }
}

pub type BlackboardPtr = Arc<RwLock<Blackboard>>;
pub type BlackboardDataPtr = Arc<RwLock<BlackboardData>>;
pub type EntryPtr = Arc<Mutex<Entry>>;

impl Blackboard {
    /// Creates a Blackboard with no parent and returns it as a `BlackboardPtr`.
    pub fn new() -> Blackboard {
        Self {
            parent_bb: Box::new(None),
            data: Arc::new(RwLock::new(BlackboardData {
                storage: HashMap::new(),
                internal_to_external: HashMap::new(),
                auto_remapping: false,
            })),
        }
    }

    fn create(parent_bb: Option<Blackboard>) -> Blackboard {
        Self {
            data: Arc::new(RwLock::new(BlackboardData {
                storage: HashMap::new(),
                internal_to_external: HashMap::new(),
                auto_remapping: false,
            })),
            parent_bb: Box::new(parent_bb),
        }
    }

    /// Creates a Blackboard with `parent_bb` as the parent. Returned as a new `BlackboardPtr`.
    pub fn with_parent(parent_bb: &Blackboard) -> Blackboard {
        Self::create(Some(parent_bb.clone()))
    }

    /// Enables the Blackboard to use autoremapping when getting values from
    /// the parent Blackboard. Only uses autoremapping if there's no matching
    /// explicit remapping rule.
    pub fn enable_auto_remapping(&mut self, use_remapping: bool) {
        self.data.write().auto_remapping = use_remapping;
    }

    /// Adds remapping rule for Blackboard. Maps from `internal` (this Blackboard)
    /// to `external` (a parent Blackboard)
    pub fn add_subtree_remapping(&mut self, internal: String, external: String) {
        self.data
            .write()
            .internal_to_external
            .insert(internal, external);
    }

    /// Get an Rc to the Entry
    fn get_entry(&mut self, key: &str) -> Option<EntryPtr> {
        let mut blackboard = self.data.write();

        // Try to get the key
        if let Some(entry) = blackboard.storage.get(key) {
            return Some(Arc::clone(entry));
        }
        // Couldn't find key. Try remapping if we have a parent
        else if let Some(parent_bb) = self.parent_bb.as_mut() {
            if let Some(new_key) = blackboard.internal_to_external.get(key) {
                // Return the value of the parent's `get()`
                let parent_entry = parent_bb.get_entry(new_key);

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
                return parent_bb.get_entry(key);
            }
        }

        // No matches
        None
    }

    /// Internal method that just tries to get value at key. If the stored
    /// type is not T, return None
    fn __get_no_string<T>(&mut self, key: &str) -> Option<EntryGuard<T>>
    where
        T: Any,
    {
        self.get_entry(key).and_then(|entry| {
            // let guard = ;

            EntryGuard::create(entry)

            // // Try to downcast directly to T
            // entry.downcast_ref::<T>().cloned()
        })
    }

    /// Internal method that tries to get the value at key as a
    /// `String` or `&str`, returning an owned type
    fn __get_string(&mut self, key: &str) -> Option<String> {
        self.get_entry(key).and_then(|entry| {
            let entry_lock = entry.lock();

            // If value is a String or &str, try to call `FromString` to convert to T
            entry_lock
                .downcast_ref::<String>()
                .map(ToString::to_string)
                .or_else(|| entry_lock.downcast_ref::<&str>().map(ToString::to_string))
        })
    }

    /// Internal method that tries to get the value at key, but only works
    /// if it's a String/&str, then tries FromString to convert it to T. Treats
    /// the `Entry` as a `Entry::Generic`
    fn __get_allow_string<T>(&mut self, key: &str) -> Option<EntryGuard<T>>
    where
        T: Any + FromString + Send,
    {
        // Try to get the key
        if let Some(entry) = self.get_entry(key) {
            let value = self.__get_string(key)?;

            // Try to parse String into T
            if let Ok(value) = <String as ParseStr<T>>::parse_str(&value) {
                // Update value with the value type instead of just a string
                let mut t = entry.lock();
                t.0 = Box::new(value);

                // Release the lock
                drop(t);

                return EntryGuard::create(entry);
            }
        }

        // No matches
        None
    }

    /// Tries to return an owned copy of the value at `key`. The type `T` must
    /// implement [`FromString`] when calling this method; it will try to convert
    /// from `String`/`&str` if there's an entry at `key` but it is not
    /// of type `T`. If it does convert it successfully, it will replace
    /// the existing value with `T` so converting from the string type
    /// won't be needed next time.
    ///
    /// If you want to get an entry that has a type that doesn't implement
    /// `FromString`, use [`Blackboard::new`] instead.
    ///
    /// The `Blackboard` tries a few things when reading a `key`:
    /// - First it checks if it can find `key`:
    ///     - Check itself for `key`
    ///     - If it doesn't exist, if`self` has a parent `Blackboard`, it checks for key remapping
    ///         - If a remapping rule exists for `key`, use the remapped `key`
    ///         - If `auto_remapping` is enabled, it uses `key` directly
    ///     - Return `None` if none of the above work
    /// - If a value is matched, attempt to coerce the value to `T`. If it couldn't be coerced to `T`:
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
    /// use behaviortree_rs::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("foo", 132u32);
    /// assert_eq!(blackboard.get::<u32>("foo"), Some(132u32));
    ///
    /// blackboard.set("bar", "100");
    ///
    /// assert_eq!(blackboard.get::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard.get::<u32>("bar"), Some(100u32));
    /// ```
    pub fn get<T>(&mut self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone + FromString + Send,
    {
        self.get_ref(key)
            .map(|val: EntryGuard<T>| val.clone_consume())
    }

    /// Works the same as `Blackboard::get`, except it doesn't clone the value.
    /// Instead, it returns a [`BlackboardValue<T>`] which wraps the `MutexGuard`
    /// and provides an immutable reference to `T`.
    ///
    /// # Locking
    /// Until the returned value is dropped or consumed, it holds a lock on the
    /// `Mutex` for the entry at `key`. **If you do not release the lock, you may get
    /// unexpected behavior, such as deadlocks.** The lock is only held on the
    /// entry at `key`, not the entire `Blackboard`.
    ///
    /// There are two ways to release the lock:
    /// - Call `drop` on the value
    /// - Call [`BlackboardValue::clone_consume`] which will clone `T`, consuming
    ///     the value and dropping the lock.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("bar", "100");
    ///
    /// let bar_ref = blackboard.get_ref::<String>("bar");
    /// assert!(bar_ref.is_some());
    /// let bar_ref = bar_ref.unwrap();
    /// // Access the inner value using `Deref::deref`
    /// assert_eq!(*bar_ref, String::from("100"));
    /// // or calling methods on the inner type via auto-deref
    /// assert_eq!(bar_ref.as_str(), "100");
    ///
    /// // Clone the value, consuming and dropping the lock
    /// let bar_ref_owned = bar_ref.clone_consume();
    ///
    /// let bar_ref = blackboard.get_ref::<u32>("bar");
    /// assert!(bar_ref.is_some());
    /// let bar_ref = bar_ref.unwrap();
    /// assert_eq!(*bar_ref, 100u32);
    ///
    /// // Drop the lock without cloning
    /// drop(bar_ref);
    /// ```
    ///
    /// The lock is only held on the entry itself, not the entire `Blackboard`,
    /// so you can hold multiple values at the same time
    ///
    /// ```
    /// blackboard.set("foo", 123u32);
    ///
    /// let foo = blackboard.get_ref::<u32>("foo").unwrap();
    /// let bar = blackboard.get_ref::<u32>("bar").unwrap();
    ///
    /// assert_ne!(*foo, *bar);
    ///
    /// // Don't forget to drop the locks once you no longer need them
    /// drop(foo);
    /// drop(bar);
    /// ```
    pub fn get_ref<T>(&mut self, key: impl AsRef<str>) -> Option<EntryGuard<T>>
    where
        T: Any + FromString + Send,
    {
        // Try without parsing string first, then try with parsing string
        self.__get_no_string(key.as_ref())
            .or_else(|| self.__get_allow_string(key.as_ref()))
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
    /// use behaviortree_rs::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("foo", 132u32);
    /// assert_eq!(blackboard.get_exact::<u32>("foo"), Some(132u32));
    ///
    /// blackboard.set("bar", "100");
    ///
    /// assert_eq!(blackboard.get_exact::<&str>("bar"), Some("100"));
    /// assert_eq!(blackboard.get_exact::<String>("bar"), None);
    /// assert_eq!(blackboard.get_exact::<u32>("bar"), None);
    /// ```
    pub fn get_exact<T>(&mut self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone,
    {
        self.__get_no_string(key.as_ref())
            .map(|val: EntryGuard<T>| val.clone_consume())
    }

    /// Works the same as [`Blackboard::get_exact`], except it doesn't clone the value.
    /// See [`Blackboard::get_ref`] for details about the difference
    pub fn get_exact_ref<T>(&mut self, key: impl AsRef<str>) -> Option<EntryGuard<T>>
    where
        T: Any,
    {
        self.__get_no_string(key.as_ref())
    }

    /// Sets the `value` in the Blackboard at `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::create();
    ///
    /// blackboard.set("foo", 132u32);
    /// assert_eq!(blackboard.get::<u32>("foo"), Some(132u32));
    ///
    /// blackboard.set("bar", "100");
    ///
    /// assert_eq!(blackboard.get::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard.get::<u32>("bar"), Some(100u32));
    /// ```
    pub fn set<T: Any + Send + 'static>(&mut self, key: impl AsRef<str>, value: T) {
        let key = key.as_ref();

        let blackboard = self.data.write();

        if let Some(entry) = blackboard.storage.get(key) {
            let mut entry = entry.lock();

            // Overwrite value of existing entry
            entry.0 = Box::new(value);
        } else {
            drop(blackboard);
            let entry = self.create_entry(&key);

            let mut entry = entry.lock();

            // Set value of new entry
            entry.0 = Box::new(value);
        }
    }

    fn create_entry<'a>(&'a mut self, key: &'a (impl AsRef<str> + Sync)) -> EntryPtr {
        let entry;

        let mut blackboard = self.data.write();

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
                .create_entry(remapped_key);
        }
        // Use autoremapping
        else if blackboard.auto_remapping && self.parent_bb.is_some() {
            entry = (*self.parent_bb).as_mut().unwrap().create_entry(key);
        }
        // No remapping or no parent blackboard
        else {
            // Create an entry with an empty placeholder value
            entry = Arc::new(Mutex::new(Entry(Box::new(()))));
        }

        blackboard
            .storage
            .insert(key.as_ref().to_string(), Arc::clone(&entry));
        entry
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self {
            data: Arc::new(RwLock::new(BlackboardData::default())),
            parent_bb: Box::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // TODO: add other tests

    #[rstest]
    fn no_remapping() {
        // With no remapping

        let mut root_bb = Blackboard::new();
        let mut left_bb = Blackboard::with_parent(&root_bb);
        let mut right_bb = Blackboard::with_parent(&root_bb);

        left_bb.set("foo", 123u32);

        assert!(left_bb.get::<u32>("foo").is_some());
        // These two should be none because remapping is not enabled
        assert!(right_bb.get::<u32>("foo").is_none());
        assert!(root_bb.get::<u32>("foo").is_none());
    }

    #[rstest]
    fn auto_remapping() {
        // With autoremapping

        let mut root_bb = Blackboard::new();
        let mut left_bb = Blackboard::with_parent(&root_bb);
        let mut right_bb = Blackboard::with_parent(&root_bb);

        root_bb.enable_auto_remapping(true);
        left_bb.enable_auto_remapping(true);
        right_bb.enable_auto_remapping(true);

        left_bb.set("foo", 123u32);

        assert_eq!(left_bb.get::<u32>("foo"), Some(123));
        assert_eq!(right_bb.get::<u32>("foo"), Some(123));
        assert_eq!(root_bb.get::<u32>("foo"), Some(123));
    }

    #[rstest]
    fn custom_remapping() {
        // With custom remapping
        let mut root_bb = Blackboard::new();
        let mut left_bb = Blackboard::with_parent(&root_bb);
        let mut right_bb = Blackboard::with_parent(&root_bb);

        right_bb.add_subtree_remapping(String::from("foo"), String::from("bar"));
        left_bb.add_subtree_remapping(String::from("foo"), String::from("bar"));

        left_bb.set("foo", 123u32);

        assert_eq!(left_bb.get::<u32>("foo"), Some(123));
        assert_eq!(right_bb.get::<u32>("foo"), Some(123));
        assert_eq!(root_bb.get::<u32>("bar"), Some(123));
    }

    #[test]
    fn remapping() {
        // No remapping

        let mut root_bb = Blackboard::new();
        let mut child_bb = Blackboard::with_parent(&root_bb);

        root_bb.set("foo", 123u32);

        assert!(child_bb.get::<u32>("foo").is_none());

        // Auto remapping

        let mut root_bb = Blackboard::new();
        let mut child1_bb = Blackboard::with_parent(&root_bb);
        let mut child2_bb = Blackboard::with_parent(&child1_bb);
        let mut child3_bb = Blackboard::with_parent(&child2_bb);

        child1_bb.enable_auto_remapping(true);
        child2_bb.enable_auto_remapping(true);
        child3_bb.enable_auto_remapping(true);

        root_bb.set("foo", 123u32);

        assert_eq!(child1_bb.get::<u32>("foo"), Some(123));
        assert_eq!(child2_bb.get::<u32>("foo"), Some(123));
        assert_eq!(child3_bb.get::<u32>("foo"), Some(123));

        // Custom remapping

        let mut root_bb = Blackboard::new();
        let mut child1_bb = Blackboard::with_parent(&root_bb);
        let mut child2_bb = Blackboard::with_parent(&child1_bb);
        let mut child3_bb = Blackboard::with_parent(&child2_bb);

        child1_bb.add_subtree_remapping(String::from("child1"), String::from("root"));
        child2_bb.add_subtree_remapping(String::from("child2"), String::from("child1"));
        child3_bb.add_subtree_remapping(String::from("child3"), String::from("child2"));

        root_bb.set("root", 123u32);

        assert_eq!(child1_bb.get::<u32>("child1"), Some(123));
        assert_eq!(child2_bb.get::<u32>("child2"), Some(123));
        assert_eq!(child3_bb.get::<u32>("child3"), Some(123));
        assert_eq!(child3_bb.get::<u32>("foo"), None);
    }

    #[rstest]
    fn type_matching() {
        let mut bb = Blackboard::new();

        bb.set("foo", 123u32);

        assert!(bb.get::<u32>("foo").is_some());
        assert!(bb.get::<String>("foo").is_none());
        assert!(bb.get::<f32>("foo").is_none());
    }

    #[rstest]
    fn custom_type() {
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

        let mut bb = Blackboard::new();

        let custom_value = CustomEntry {
            foo: 123,
            bar: String::from("bar"),
        };

        bb.set("custom", custom_value.clone());
        bb.set("custom_str", String::from("123,bar"));
        bb.set("custom_str_malformed", String::from("not an int,bar"));

        assert_eq!(bb.get::<CustomEntry>("custom"), Some(custom_value.clone()));

        // Check parse from String
        assert_eq!(bb.get::<CustomEntry>("custom_str"), Some(custom_value));
        let val = bb.get::<CustomEntry>("custom_str_malformed");
        // Check it returns None if it cannot be parsed
        assert!(val.is_none());
    }
}
