use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::Arc,
};

use ouroboros::self_referencing;
use parking_lot::{Mutex, MutexGuard, RwLock};

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
    /// assert_eq!("{value}".strip_bb_pointer(), Some("value"));
    /// ```
    fn strip_bb_pointer(&self) -> Option<&str>;
    fn is_bb_pointer(&self) -> bool;
}

impl<T> BlackboardString for T
where
    T: AsRef<str> + Clone,
{
    fn strip_bb_pointer(&self) -> Option<&str> {
        let str_ref = self.as_ref();

        // Try to remove wrapping braces, returns None if either one is missing
        str_ref
            .strip_prefix("{")
            .and_then(|val| val.strip_suffix("}"))
    }

    fn is_bb_pointer(&self) -> bool {
        let str_ref = self.as_ref();
        str_ref.starts_with("{") && str_ref.ends_with("}")
    }
}

/// Struct that stores arbitrary data in a `HashMap<String, Box<dyn Any + Send>>`. Note the
/// stored data type _must_ implement `Send`.
///
/// # Examples
///
/// Create a root-level `Blackboard` using [`Blackboard::new`].
///
/// ```
/// use behaviortree_rs::prelude::*;
///
/// // Create a root-level Blackboard
/// let bb = Blackboard::new();
///
/// // Create a child Blackboard
/// let child = Blackboard::with_parent(&bb);
/// ```
#[derive(Debug, Clone)]
pub struct Blackboard {
    data: Arc<RwLock<BlackboardData>>,
    parent: Option<Box<Blackboard>>,
}

/// Private struct that holds all blackboard data
#[derive(Debug, Default)]
struct BlackboardData {
    storage: HashMap<String, EntryPtr>,
    /// Manual remapping rules from this blackboard to the parent
    internal_to_external: HashMap<String, String>,
    /// Whether to use auto-remapping
    auto_remapping: bool,
}

/// Convenience type for the pointer around an [`Entry`]
type EntryPtr = Arc<Mutex<Entry>>;

/// Holds the data for a [`Blackboard`] value at a key.
#[derive(Debug)]
struct Entry(Box<dyn Any + Send>);

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

/// Self-referencing struct that holds a copied [`EntryPtr`], the locked
/// `MutexGuard` around the `Entry`, and a reference to downcasted `T` borrowed from
/// the `MutexGuard`.
#[self_referencing]
struct EntryGuardInner<T>
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

/// Locked [`Blackboard`] entry. Until this value is dropped, the lock is held
/// on the `Blackboard` entry.
///
/// Implements [`Deref`], providing access to the locked `T`.
pub struct EntryGuard<T: 'static>(EntryGuardInner<T>);

#[derive(Debug)]
pub struct EntryRef(EntryPtr);

impl EntryRef {
    pub fn downcast_ref<T>(&self) -> Option<EntryGuard<T>>
    where
        T: Any + 'static,
    {
        EntryGuard::create(Arc::clone(&self.0))
    }

    pub fn downcast_clone<T>(&self) -> Option<T>
    where
        T: Any + Clone + 'static,
    {
        self.0.lock().downcast_ref().cloned()
    }
}

impl<T> EntryGuard<T>
where
    T: 'static,
{
    /// Attempts to downcast the `Box<dyn Any + Send>` to `T`. If downcasting
    /// succeeds, wraps the value and lock into [`EntryGuard`].
    fn create(entry: EntryPtr) -> Option<Self> {
        // Check if the inner value can be downcasted directly to `T`
        let is_valid = entry.lock().downcast_ref::<T>().is_some();

        if is_valid {
            let inner = EntryGuardInner::new(
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

impl Blackboard {
    /// Creates an empty `Blackboard` with no parent.
    pub fn new() -> Blackboard {
        Self::default()
    }

    /// Create an empty [`Blackboard`], with or without a parent. Only used internally.
    fn create(parent: Option<Blackboard>) -> Blackboard {
        Self {
            data: Arc::new(RwLock::new(BlackboardData {
                storage: HashMap::new(),
                internal_to_external: HashMap::new(),
                auto_remapping: false,
            })),
            parent: parent.map(Box::new),
        }
    }

    /// Creates an empty `Blackboard` with `parent_bb` as the parent.
    pub fn with_parent(parent_bb: &Blackboard) -> Blackboard {
        Self::create(Some(parent_bb.clone()))
    }

    /// Enables the Blackboard to use autoremapping when getting values from
    /// the parent Blackboard. Only uses autoremapping if there's no matching
    /// explicit remapping rule.
    pub fn set_auto_remapping(&self, use_remapping: bool) {
        self.data.write().auto_remapping = use_remapping;
    }

    /// Adds remapping rule for Blackboard. Maps from `internal` (this Blackboard)
    /// to `external` (a parent Blackboard)
    pub fn add_subtree_remapping(&self, internal: String, external: String) {
        self.data
            .write()
            .internal_to_external
            .insert(internal, external);
    }

    /// Checks if the `Blackboard` contains a value at `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let blackboard = Blackboard::new();
    ///
    /// blackboard.set("foo", 123);
    ///
    /// assert!(blackboard.contains_key("foo"));
    ///
    /// assert!(!blackboard.contains_key("bar"));
    /// ```
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self._get_entry(key.as_ref()).is_some()
    }

    pub fn get_entry_ref(&self, key: impl AsRef<str>) -> Option<EntryRef> {
        self._get_entry(key.as_ref()).map(EntryRef)
    }

    /// Tries to return an owned copy of the value at `key`. The type `T` must
    /// implement [`FromStr`] when calling this method; it will try to convert
    /// from `String`/`&str` if there's an entry at `key` but it is not
    /// of type `T`. If it does convert it successfully, it will replace
    /// the existing value with `T` so converting from the string type
    /// won't be needed next time.
    ///
    /// If you want to get an entry that has a type that doesn't implement
    /// `FromStr`, use [`Blackboard::get_exact`] instead.
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
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::new();
    ///
    /// blackboard.set("foo", 132u32);
    /// assert_eq!(blackboard.get::<u32>("foo"), Some(132u32));
    ///
    /// blackboard.set("bar", "100");
    ///
    /// assert_eq!(blackboard.get::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard.get::<u32>("bar"), Some(100u32));
    /// ```
    pub fn get<T>(&self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone + FromStr + Send,
    {
        self.get_ref(key)
            .map(|val: EntryGuard<T>| val.clone_consume())
    }

    /// Works the same as [`Blackboard::get`], except it doesn't clone the value.
    /// Instead, it returns a [`EntryGuard<T>`] which wraps the `MutexGuard`
    /// and provides an immutable reference to `T`.
    ///
    /// # Locking
    /// Until the returned value is dropped or consumed, it holds a lock on the
    /// `Mutex` for the entry at `key`. **If you do not release the lock, you may get
    /// unexpected behavior, such as deadlocks.**
    ///
    /// The lock is only held on the entry at `key`, not the entire `Blackboard`.
    ///
    /// There are two ways to release the lock:
    /// - Call `drop` on the value
    /// - Call [`EntryGuard::clone_consume`] which will clone `T`, consuming
    ///   the value and dropping the lock.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let mut blackboard = Blackboard::new();
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
    /// use behaviortree_rs::prelude::*;
    ///
    /// let mut blackboard = Blackboard::new();
    ///
    /// blackboard.set("bar", "100");
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
    pub fn get_ref<T>(&self, key: impl AsRef<str>) -> Option<EntryGuard<T>>
    where
        T: Any + FromStr + Send,
    {
        // Try without parsing string first, then try with parsing string
        self._get_value(key.as_ref())
            .or_else(|| self._parse_from_string(key.as_ref()))
    }

    /// Version of `get<T>` that does _not_ try to convert from string if the type
    /// doesn't match. This method has the benefit of not requiring the trait
    /// [`FromStr`], which allows you to avoid implementing the trait for
    /// types that don't need it or it's impossible to represent the data
    /// type as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::new();
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
    pub fn get_exact<T>(&self, key: impl AsRef<str>) -> Option<T>
    where
        T: Any + Clone,
    {
        self._get_value(key.as_ref())
            .map(|val: EntryGuard<T>| val.clone_consume())
    }

    /// Works the same as [`Blackboard::get_exact`], except it doesn't clone the value.
    /// See [`Blackboard::get_ref`] for details about the difference.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    /// use behaviortree_rs::blackboard::EntryGuard;
    ///
    /// let mut blackboard = Blackboard::new();
    ///
    /// blackboard.set("bar", 100u32);
    ///
    /// let bar_ref: Option<EntryGuard<u32>> = blackboard.get_exact_ref::<u32>("bar");
    /// assert!(bar_ref.is_some());
    /// let bar_ref = bar_ref.unwrap();
    /// // Access the inner value using `Deref::deref`
    /// assert_eq!(*bar_ref, 100u32);
    ///
    /// // Clone the value, consuming and dropping the lock
    /// let bar_ref_owned = bar_ref.clone_consume();
    /// ```
    pub fn get_exact_ref<T>(&self, key: impl AsRef<str>) -> Option<EntryGuard<T>>
    where
        T: Any,
    {
        self._get_value(key.as_ref())
    }

    /// Sets the `value` in the Blackboard at `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::blackboard::Blackboard;
    ///
    /// let mut blackboard = Blackboard::new();
    ///
    /// blackboard.set("foo", 132u32);
    /// assert_eq!(blackboard.get::<u32>("foo"), Some(132u32));
    ///
    /// blackboard.set("bar", "100");
    ///
    /// assert_eq!(blackboard.get::<String>("bar"), Some(String::from("100")));
    /// assert_eq!(blackboard.get::<u32>("bar"), Some(100u32));
    /// ```
    pub fn set<T>(&self, key: impl AsRef<str>, value: T)
    where
        T: Any + Send + 'static,
    {
        self._update_or_create_entry(key.as_ref(), Box::new(value));
    }

    /// Internal method that just tries to get value at key. If the stored
    /// type is not `T`, return `None`
    fn _get_value<T>(&self, key: &str) -> Option<EntryGuard<T>>
    where
        T: Any,
    {
        self._get_entry(key)
            .and_then(|entry| EntryGuard::create(entry))
    }

    /// Internal method that tries to get the value at key as a
    /// `String` or `&str`, returning an owned type
    fn _get_as_string(&self, key: &str) -> Option<String> {
        self._get_entry(key).and_then(|entry| {
            let entry_lock = entry.lock();

            // Try to downcast value to either String or &str and return as String
            entry_lock
                .downcast_ref::<String>()
                .map(|val| val.as_str())
                .or_else(|| entry_lock.downcast_ref::<&str>().copied())
                .map(ToString::to_string)
        })
    }

    /// Internal method that tries to get the value at key, but only works
    /// if it's a `String`/`&str`, then tries [`FromStr::from_str`] to convert it to T.
    fn _parse_from_string<T>(&self, key: &str) -> Option<EntryGuard<T>>
    where
        T: Any + FromStr + Send,
    {
        // Try to get the key
        if let Some(entry) = self._get_entry(key) {
            let value = self._get_as_string(key)?;

            // Try to parse String into T
            if let Ok(value) = <T as FromStr>::from_str(&value) {
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

    /// Try to get a cloned [`EntryPtr`] for `key`
    fn _get_entry(&self, key: &str) -> Option<EntryPtr> {
        let mut blackboard = self.data.write();

        // Try to get the key
        if let Some(entry) = blackboard.storage.get(key) {
            return Some(Arc::clone(entry));
        }
        // Couldn't find key. Try remapping if we have a parent
        else if let Some(parent) = self.parent.as_ref() {
            if let Some(new_key) = blackboard.internal_to_external.get(key) {
                // Return the value of the parent's `get()`
                let parent_entry = parent._get_entry(new_key);

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
                return parent._get_entry(key);
            }
        }

        // No matches
        None
    }

    /// Updates the value at `key`, or creates a new [`Entry`].
    fn _update_or_create_entry(&self, key: &str, value: Box<dyn Any + Send>) {
        let mut blackboard = self.data.write();

        // If the entry already exists
        if let Some(existing_entry) = blackboard.storage.get(key) {
            existing_entry.lock().0 = value;
        } else if let Some(parent) = self.parent.as_ref() {
            // Use explicit remapping rule
            if let Some(remapped_key) = blackboard.internal_to_external.get(key) {
                parent._update_or_create_entry(remapped_key, value);
            }
            // Use autoremapping
            else if blackboard.auto_remapping {
                parent._update_or_create_entry(key, value)
            }
            // No remapping
            else {
                // Create a new entry
                let entry = Arc::new(Mutex::new(Entry(value)));

                blackboard
                    .storage
                    .insert(key.to_string(), Arc::clone(&entry));
            }
        }
        // No parent blackboard
        else {
            // Create a new entry
            let entry = Arc::new(Mutex::new(Entry(value)));

            blackboard
                .storage
                .insert(key.to_string(), Arc::clone(&entry));
        }
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self {
            data: Arc::new(RwLock::new(BlackboardData::default())),
            parent: None,
        }
    }
}
