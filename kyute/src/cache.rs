use crate::{call_key::CallKey, data::Data};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap,
    },
    hash::{Hash, Hasher},
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
    args_hash: u64,
    data: Box<dyn Any>,
    dirty: Cell<bool>,
}

impl CacheEntry {
    fn new<T: Any>(data: T, args_hash: u64, depends_on: Vec<CallKey>) -> CacheEntry {
        CacheEntry {
            depends_on,
            args_hash,
            data: Box::new(data),
            dirty: Cell::new(false),
        }
    }

    /*fn add_dependency(&mut self, key: CallKey) {
        if !self.depends_on.contains(&key) {
            self.depends_on.push(key);
        }
    }*/

    fn mark_dirty(&self) {
        self.dirty.set(true);
    }

    fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Sets the contents of the cache entry, and resets the dirty flag.
    fn update<T: Any>(&mut self, data: T, args_hash: u64, depends_on: Vec<CallKey>) {
        assert_eq!(TypeId::of::<T>(), self.data.type_id());
        self.data = Box::new(data);
        self.args_hash = args_hash;
        self.depends_on = depends_on;
        self.dirty.set(false);
    }
}

pub(crate) struct Cache {
    // keys: RefCell<HashMap<CallKey, CacheEntryKey>>,
    entries: RefCell<HashMap<CallKey, CacheEntry>>,
    /// Keeps track of reentrant calls to `cache`, for dependency tracking.
    current_nested_entries: RefCell<Vec<CallKey>>,
}

impl Cache {
    pub(crate) fn new() -> Cache {
        Cache {
            entries: RefCell::new(Default::default()),
            current_nested_entries: RefCell::new(vec![]),
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
            eprintln!("- {:?}: depends on {:?}", key, entry.depends_on);
        }
    }

    pub(crate) fn set_value<T>(&mut self, key: CallKey, _value: T) {
        //let entry = self.entries.borrow_mut().get_mut(&key).unwrap();
        let entries = self.entries.borrow();
        let entry = entries.get(&key).unwrap();
        for dep in entry.depends_on.iter() {
            self.dirty_entry_recursive(*dep);
        }
        // FIXME: this can result in a large number of hash map accesses on every state change.
        // (the deeper the state, the more hash map accesses)
    }


    /// Runs the function that computes the value of a cache entry, and returns the value itself
    /// and the keys of all its direct dependencies.
    fn invoke_cache_init_fn<T, Args>(
        &self,
        key: CallKey,
        args: Args,
        f: impl FnOnce(&Args) -> T,
    ) -> (T, Vec<CallKey>)
    where
        T: Any + Data,
        Args: Hash,
    {
        let mut current_nested_entries = self.current_nested_entries.take();
        let data = f(&args);
        current_nested_entries.push(key);
        let deps = self.current_nested_entries.replace(current_nested_entries);
        (data, deps)
    }

    /// Calls to this function are reentrant: this function can be called from `f`.
    pub(crate) fn cache<T, Args>(
        &self,
        key: CallKey,
        args: Args,
        f: impl FnOnce(&Args) -> T,
        location: Option<&'static Location>,
    ) -> T
    where
        T: Any + Data,
        Args: Hash,
    {
        let args_hash = {
            let mut s = DefaultHasher::new();
            args.hash(&mut s);
            s.finish()
        };

        if let Some(entry) = self.entries.borrow().get(&key) {
            if entry.args_hash == args_hash && !entry.is_dirty() {
                return entry.data.downcast_ref::<T>().unwrap().clone();
            }
        }

        let (data, depends_on) = self.invoke_cache_init_fn(key, args, f);

        match self.entries.borrow_mut().entry(key) {
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                entry.update(data.clone(), args_hash, depends_on);
                false
            }
            Entry::Vacant(entry) => {
                entry.insert(CacheEntry::new(data.clone(), args_hash, depends_on));
                true
            }
        };

        data
    }
}
