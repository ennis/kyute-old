use crate::{call_key::CallKey, data::Data};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap,
    },
    hash::{Hash, Hasher},
    marker::PhantomData,
    panic::Location,
    sync::Arc,
};
use tracing::trace;

// - each cache entry can be uniquely identified by its call key
// - if calling from a generic get<T> function, if the call key is the same, the type is known to be T
// - it's important that the lookup be fast

slotmap::new_key_type! {
    pub struct CacheEntryKey;
}

struct CacheEntry {
    depends_on: Vec<CallKey>,
    args_hash: Option<u64>,
    value: Box<dyn Any>,
    dirty: Cell<bool>,
    // Only for debugging
    location: Option<&'static Location<'static>>,
}

impl CacheEntry {
    ///
    fn is_mutable(&self) -> bool {
        self.args_hash.is_some()
    }

    fn mark_dirty(&self) {
        self.dirty.set(true);
    }

    fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /*/// Sets the contents of the cache entry, and resets the dirty flag.
    fn update<T: Any>(&mut self, value: T, args_hash: u64, depends_on: Vec<CallKey>) {
        assert_eq!(
            TypeId::of::<T>(),
            self.value.type_id(),
            "unexpected type of cache entry value"
        );
        self.value = Box::new(value);
        self.args_hash = Some(args_hash);
        self.depends_on = depends_on;
        self.dirty.set(false);
    }*/
}

pub(crate) struct Cache {
    // keys: RefCell<HashMap<CallKey, CacheEntryKey>>,
    entries: RefCell<HashMap<CallKey, CacheEntry>>,
    /// Keeps track of reentrant calls to `cache`, for dependency tracking.
    current_dependencies: RefCell<Vec<CallKey>>,
}

impl Cache {
    pub(crate) fn new() -> Cache {
        Cache {
            entries: RefCell::new(Default::default()),
            current_dependencies: RefCell::new(vec![]),
        }
    }

    fn dirty_entry_recursive(&self, key: CallKey) {
        let entries = self.entries.borrow();
        let entry = entries.get(&key).unwrap();
        entry.mark_dirty();
        for dep in entry.depends_on.iter() {
            self.dirty_entry_recursive(*dep);
        }
    }

    pub(crate) fn dump(&self) {
        eprintln!("====== Cache entries: ======");
        for (key, entry) in self.entries.borrow().iter() {
            if let Some(location) = entry.location {
                eprintln!(
                    "- [{}]({:?}): depends on {:?}",
                    location, key, entry.depends_on
                );
            } else {
                eprintln!("- {:?} @ <unknown>: depends on {:?}", key, entry.depends_on);
            }
        }
    }

    /// Sets the value of a modifiable state entry.
    pub(crate) fn set_state<T: 'static>(&self, key: CallKey, value: T) {
        // update the value stored in the entry
        {
            let mut entries = self.entries.borrow_mut();
            let entry = entries.get_mut(&key).expect("cache entry not found");
            assert!(
                entry.args_hash.is_some(),
                "attempted to set the value of an immutable cache entry"
            );
            *entry
                .value
                .downcast_mut::<T>()
                .expect("cache entry type mismatch") = value;
        }

        // recursively mark dependent entries as dirty
        //
        // FIXME: this can result in a large number of hash map accesses on every state change.
        // (the deeper the state, the more hash map accesses)
        // Replace with something for efficient?
        let entries = self.entries.borrow();
        let entry = entries.get(&key).unwrap();
        for dep in entry.depends_on.iter() {
            self.dirty_entry_recursive(*dep);
        }
    }

    /// Runs the function that computes the value of a cache entry, and returns the value itself
    /// and the keys of all its direct dependencies.
    fn invoke_cache_init_fn<T: Any + Clone>(
        &self,
        key: CallKey,
        f: impl FnOnce() -> T,
    ) -> (T, Vec<CallKey>) {
        // invoke_cache_init_fn should be reentrant, and `current_dependencies` may be affected,
        // so save its current value.
        let mut current_dependencies = self.current_dependencies.take();
        let value = f();
        // restore `current_dependencies`, but also add this cache entry to the list, as a dependency
        // of the parent entry.
        current_dependencies.push(key);
        let deps = self.current_dependencies.replace(current_dependencies);
        (value, deps)
    }

    pub(crate) fn cache_impl<T: Any + Clone>(
        &self,
        key: CallKey,
        args_hash: Option<u64>,
        f: impl FnOnce() -> T,
        location: Option<&'static Location<'static>>,
    ) -> T {
        // if an entry already exists and the argument hash matches, return it.
        if let Some(entry) = self.entries.borrow().get(&key) {
            assert_eq!(
                entry.args_hash.is_some(),
                entry.args_hash.is_some(),
                "existing cache entry differs in mutability"
            );
            if entry.args_hash == args_hash && !entry.is_dirty() {
                return entry
                    .value
                    .downcast_ref::<T>()
                    .expect("cache entry type mismatch")
                    .clone();
            }
        }

        let (value, depends_on) = self.invoke_cache_init_fn(key, f);

        match self.entries.borrow_mut().entry(key) {
            Entry::Occupied(mut entry) => {
                // update the existing cache entry with the new value and hash, and reset its dirty
                // flag. also make sure that the type is correct.
                let entry = entry.get_mut();
                *entry
                    .value
                    .downcast_mut::<T>()
                    .expect("cache entry type mismatch") = value.clone();
                entry.args_hash = args_hash;
                entry.depends_on = depends_on;
                entry.dirty.set(false);
            }
            Entry::Vacant(entry) => {
                // insert a fresh entry
                entry.insert(CacheEntry {
                    depends_on,
                    args_hash,
                    value: Box::new(value.clone()),
                    dirty: Cell::new(false),
                    location,
                });
            }
        };

        value
    }

    pub(crate) fn cache<T, Args>(
        &self,
        key: CallKey,
        args: Args,
        f: impl FnOnce(&Args) -> T,
        location: Option<&'static Location<'static>>,
    ) -> T
    where
        T: Any + Clone,
        Args: Hash,
    {
        let args_hash = {
            let mut s = DefaultHasher::new();
            args.hash(&mut s);
            s.finish()
        };

        self.cache_impl(key, Some(args_hash), move || f(&args), location)
    }

    pub(crate) fn cache_state<T:Any+Clone>(
        &self,
        key: CallKey,
        init: impl FnOnce() -> T,
        location: Option<&'static Location<'static>>,
    ) -> T {
        self.cache_impl(key, None, init, location)
    }
}
