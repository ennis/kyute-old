use crate::{
    call_key::{CallKey, CallKeyStack},
    data::Data,
};
use slotmap::SlotMap;
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap,
    },
    convert::TryInto,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::ManuallyDrop,
    panic::Location,
    sync::Arc,
};
use thiserror::Error;
use tracing::trace;

// - each cache entry can be uniquely identified by its call key
// - if calling from a generic get<T> function, if the call key is the same, the type is known to be T
// - it's important that the lookup be fast

slotmap::new_key_type! {
    pub struct GroupKey;
}

struct Group {
    parent: Option<GroupKey>,
    dirty: bool,
}

/// Error related to state entries.
#[derive(Error, Debug)]
pub enum CacheEntryError {
    #[error("state entry not found")]
    EntryNotFound,
    #[error("no value in state entry")]
    VacantEntry,
    #[error("state entry already contains a value")]
    OccupiedEntry,
    #[error("type mismatch")]
    TypeMismatch,
}

// T.tt.... | Slpd.... | E....... | V.tyvvvv | V.tyvvvv

// Simple tagged cached value:
// 32 B for the tag
// 32 B for start group
// 32 B for the value
// 32 B for end group
// == 128 B for a single cache entry

// 32b slots
// If possible, store cached values inline: need typeid (8b) + pointer or value (16b) => total 32b per entry
// For, say, 3500 UI elements, that's 448 kB of pointers

// the most common case will be memoized entries:
// => a call key, a certain number of values (let's say 2 on average), and the cached value.
// => values should be stored inline if small, to avoid indirections
// (meh: simply using a struct instead of individual params can result in poorer perfs...)
// => bag all params in a struct, stuff it in the cache, now we have only one value
// if bigger than, say, 16b, it incurs a

// How about an arena allocator?
//

enum Slot {
    /// Marks the start of a group.
    /// Contains the length of the group including this slot and the `GroupEnd` marker.
    StartGroup {
        // 4 + 4 + 8 + 8
        key: CallKey,
        group_key: GroupKey,
        len: u32,
    },
    /// Marks the end of a scope.
    EndGroup,
    /// Holds a cached value.
    Value { key: CallKey, value: Box<dyn Any> },
    /// Placeholder for a not-yet-written value
    Placeholder { key: CallKey },
}

impl Slot {
    fn update_group_len(&mut self, new_len: usize) {
        let new_len: u32 = new_len.try_into().unwrap();
        match self {
            Slot::StartGroup { len, .. } => {
                *len = new_len;
            }
            _ => {
                panic!("expected group start")
            }
        }
    }
}

pub struct CacheInner {
    slots: Vec<Slot>,
    group_map: SlotMap<GroupKey, Group>,
    revision: usize,
}

impl CacheInner {
    pub fn new() -> CacheInner {
        let mut group_map = SlotMap::with_key();
        let root_group_key = group_map.insert(Group {
            parent: None,
            dirty: false,
        });
        CacheInner {
            slots: vec![
                Slot::StartGroup {
                    key: CallKey(0),
                    group_key: root_group_key,
                    len: 2,
                },
                Slot::EndGroup,
            ],

            revision: 0,
            group_map,
        }
    }

    /// Invalidates a cache entry and all dependents.
    pub fn invalidate(&mut self, token: CacheInvalidationToken) {
        self.invalidate_group(token.key);
    }

    /*pub fn value_mut<T: 'static>(&mut self, key: ValueEntryKey) -> &mut T {
        let slot_index = *self
            .mutable_values
            .get(key)
            .expect("invalid mutable value key");
        match &mut self.slots[slot_index] {
            Slot::Value(entry) => entry
                .downcast_mut()
                .expect("type mismatch")
                .get_mut()
                .expect("entry was vacant"),
            _ => {
                panic!("unexpected entry type")
            }
        }
    }*/

    fn invalidate_group(&mut self, group_key: GroupKey) {
        if !self.group_map.contains_key(group_key) {
            tracing::warn!("invalidate_group: no such group");
            return;
        }
        let group = &mut self.group_map[group_key];
        group.dirty = true;
        if let Some(parent) = group.parent {
            self.invalidate_group(parent);
        }
    }

    pub fn dump(&self, current_position: usize) {
        for (i, s) in self.slots.iter().enumerate() {
            if i == current_position {
                eprint!("* ");
            } else {
                eprint!("  ");
            }
            match s {
                Slot::StartGroup {
                    key,
                    len,
                    group_key,
                } => {
                    let group = &self.group_map[*group_key];
                    eprintln!(
                        "{:3} StartGroup key={:?} len={} (end={}) group_key={:?} group_parent={:?} dirty={}",
                        i,
                        key,
                        *len,
                        i + *len as usize - 1,
                        group_key,
                        group.parent,
                        group.dirty,
                    )
                }
                Slot::EndGroup => {
                    eprintln!("{:3} EndGroup", i)
                }
                Slot::Value { key, value } => {
                    eprintln!("{:3} Value      key={:?} {:?}", i, key, value.type_id())
                }
                Slot::Placeholder { key } => {
                    eprintln!("{:3} Placeholder key={:?}", i, key);
                }
            }
        }
    }
}

/// Used to update a cache in a composition context.
pub struct CacheWriter {
    /// The cache being updated
    cache: CacheInner,
    /// Current writing position
    pos: usize,
    /// return index
    group_stack: Vec<usize>,
}

impl CacheWriter {
    pub fn new(cache: CacheInner) -> CacheWriter {
        let mut writer = CacheWriter {
            cache,
            pos: 0,
            group_stack: vec![],
        };
        writer.start_group(CallKey(0));
        writer
    }

    fn parent_group_key(&self) -> Option<GroupKey> {
        if let Some(&group_start) = self.group_stack.last() {
            match self.cache.slots[group_start] {
                Slot::StartGroup { group_key, .. } => Some(group_key),
                _ => panic!("unexpected entry type"),
            }
        } else {
            None
        }
    }

    pub fn get_invalidation_token(&self) -> CacheInvalidationToken {
        CacheInvalidationToken {
            key: self.parent_group_key().unwrap(),
        }
    }

    /// Finishes writing to the cache, returns the updated cache object.
    pub fn finish(mut self) -> CacheInner {
        self.end_group();
        assert!(self.group_stack.is_empty(), "unbalanced groups");
        assert_eq!(self.pos, self.cache.slots.len());
        self.cache
    }

    /// Finds a slot with the specified key in the current group, starting from the current position.
    ///
    /// # Return value
    ///
    /// The position of the matching slot in the table, or None.
    fn find_tag_in_current_group(&self, call_key: CallKey) -> Option<usize> {
        let mut i = self.pos;
        let slots = &self.cache.slots[..];

        while i < self.cache.slots.len() {
            match slots[i] {
                Slot::StartGroup { key, len, .. } => {
                    if key == call_key {
                        return Some(i);
                    }
                    i += len as usize;
                }
                Slot::Value { key, .. } if key == call_key => {
                    return Some(i);
                }
                Slot::EndGroup => {
                    // reached the end of the current group
                    return None;
                }
                _ => {
                    i += 1;
                }
            }
        }

        // no slot found
        None
    }

    fn rotate_in_current_position(&mut self, pos: usize) {
        assert!(pos >= self.pos);
        let group_end_pos = self.group_end_position();
        assert!(pos <= group_end_pos);
        self.cache.slots[self.pos..group_end_pos].rotate_left(pos - self.pos);
    }

    fn sync(&mut self, call_key: CallKey) -> bool {
        let pos = self.find_tag_in_current_group(call_key);
        match pos {
            Some(pos) => {
                // move slots in position
                self.rotate_in_current_position(pos);
                true
            }
            None => false,
        }
    }

    fn parent_group_offset(&self) -> i32 {
        if let Some(&parent) = self.group_stack.last() {
            parent as i32 - self.pos as i32
        } else {
            0
        }
    }

    /*fn update_parent_group_offset(&mut self) {
        let parent = self.parent_group_offset();
        match &mut self.cache.slots[self.pos] {
            Slot::Tag(_) => {}
            Slot::StartGroup { parent: old_parent, .. } => {
                *old_parent = parent;
            }
            Slot::EndGroup => {}
            Slot::State(entry) => {
                entry.parent = parent;
            }
        }
    }*/

    pub fn start_group(&mut self, call_key: CallKey) -> bool {
        let key_found = self.sync(call_key);

        //let parent = self.parent_group_offset();
        let parent = self.parent_group_key();

        let dirty = if key_found {
            match self.cache.slots[self.pos] {
                Slot::StartGroup { group_key, .. } => self.cache.group_map[group_key].dirty,
                _ => panic!("unexpected slot type"),
            }
        } else {
            // insert new group - start and end markers
            let group_key = self.cache.group_map.insert(Group {
                parent,
                dirty: false,
            });
            self.cache.slots.insert(
                self.pos,
                Slot::StartGroup {
                    key: call_key,
                    group_key,
                    len: 2,
                },
            ); // 2 = initial length of group (start+end slots)
            self.cache.slots.insert(self.pos + 1, Slot::EndGroup);
            false
        };

        // enter group
        self.group_stack.push(self.pos);
        self.pos += 1;
        dirty
    }

    pub fn dump(&self) {
        eprintln!("position : {}", self.pos);
        eprintln!("stack    : {:?}", self.group_stack);
        eprintln!("slots:");
        self.cache.dump(self.pos);
    }

    fn group_end_position(&self) -> usize {
        let mut i = self.pos;

        while i < self.cache.slots.len() {
            match self.cache.slots[i] {
                Slot::EndGroup => break,
                Slot::StartGroup { len, .. } => {
                    i += len as usize;
                }
                _ => i += 1,
            }
        }

        i
    }

    pub fn end_group(&mut self) {
        // all remaining slots in the group are now considered dead in this revision:
        // - find position of group end marker
        let group_end_pos = self.group_end_position();

        // remove the extra nodes, and remove groups from the group map
        for slot in self.cache.slots.drain(self.pos..group_end_pos) {
            match slot {
                Slot::StartGroup { group_key, .. } => {
                    self.cache.group_map.remove(group_key);
                }
                _ => {}
            }
        }

        // skip GroupEnd marker
        self.pos += 1;
        // update group length
        let group_start_pos = self.group_stack.pop().expect("unbalanced groups");
        match self.cache.slots[group_start_pos] {
            Slot::StartGroup {
                ref mut len,
                group_key,
                ..
            } => {
                self.cache.group_map[group_key].dirty = false;
                *len = (self.pos - group_start_pos).try_into().unwrap();
            }
            _ => {
                panic!("expected group start")
            }
        }
    }

    /// Skips the next entry or the next group.
    pub fn skip(&mut self) {
        match self.cache.slots[self.pos] {
            Slot::StartGroup { len, .. } => {
                self.pos += len as usize;
            }
            Slot::Value { .. } | Slot::Placeholder { .. } => {
                self.pos += 1;
            }
            Slot::EndGroup => {
                // nothing to skip
            }
        }
    }

    fn skip_until_end_of_group(&mut self) {
        while !matches!(self.cache.slots[self.pos], Slot::EndGroup) {
            self.skip()
        }
    }

    fn expect_value<T: Clone + 'static>(&mut self, call_key: CallKey) -> (Option<T>, usize) {
        let slot = self.pos;
        let value = match self.cache.slots[slot] {
            Slot::Value { key, ref mut value } if key == call_key => {
                Some(value.downcast_mut::<T>().expect("unexpected type").clone())
            }
            _ => {
                // otherwise, insert a new entry
                self.cache
                    .slots
                    .insert(slot, Slot::Placeholder { key: call_key });
                None
            }
        };
        self.pos += 1;
        (value, slot)
    }

    /*/// Reserves a value slot at the current position in the cache.
    /// If there's a value at the current position, overwrites the value, otherwise inserts a placeholder.
    /// Use `set_value` with the returned index to set the value of the slot afterwards.
    fn make_placeholder(&mut self) -> usize {
        let pos = self.pos;
        match self.cache.slots[pos] {
            // if next slot is value or placeholder, overwrite
            Slot::Value(_) => {}
            Slot::Placeholder => {}
            _ => {
                // otherwise, insert a new entry
                self.cache.slots.insert(pos, Slot::Placeholder);
            }
        }
        pos
    }*/

    fn set_value<T: 'static>(&mut self, slot: usize, value: T) {
        let key = match self.cache.slots[slot] {
            Slot::Value { key, .. } => key,
            Slot::Placeholder { key } => key,
            _ => {
                panic!("must call set_value on a placeholder or value slot")
            }
        };
        self.cache.slots[slot] = Slot::Value {
            key,
            value: Box::new(value),
        };
    }

    /*pub fn tagged_compare_and_update_value<T: Data>(
        &mut self,
        call_key: CallKey,
        new_value: T,
    ) -> bool {
        if self.sync(call_key) {
            self.compare_and_update_value(new_value)
        } else {
            self.insert_value(new_value);
            true
        }
    }*/

    pub fn compare_and_update_value<T: Data>(&mut self, call_key: CallKey, new_value: T) -> bool {
        let changed = if self.sync(call_key) {
            match self.cache.slots[self.pos] {
                Slot::Value { key, ref mut value } => {
                    assert_eq!(key, call_key);
                    let value = value.downcast_mut::<T>().expect("entry type mismatch");
                    if !new_value.same(&value) {
                        *value = new_value;
                        true
                    } else {
                        false
                    }
                }
                _ => {
                    // not expecting anything else
                    panic!("unexpected slot type");
                }
            }
        } else {
            // insert entry
            self.cache.slots.insert(
                self.pos,
                Slot::Value {
                    key: call_key,
                    value: Box::new(new_value),
                },
            );
            true
        };

        self.pos += 1;
        changed
    }

    /*pub fn tagged_take_value<T: 'static>(
        &mut self,
        call_key: CallKey,
        mutable: bool,
        init: impl FnOnce() -> T,
    ) -> (usize, T) {
        if self.sync(call_key) {
            self.take_value(mutable, init)
        } else {
            let (index, entry) = self.insert_value(mutable, init());
            (index, entry.take_value().unwrap())
        }
    }

    pub fn take_value<T: 'static>(
        &mut self,
        mutable: bool,
        init: impl FnOnce() -> T,
    ) -> (usize, T) {
        let parent = self.parent_group_offset();
        match &mut self.cache.slots[self.pos] {
            Slot::Value(entry) => {
                entry.parent = parent;
                let pos = self.pos;
                let value = entry
                    .downcast_mut::<T>()
                    .expect("entry type mismatch")
                    .take_value()
                    .unwrap_or_else(init);
                self.pos += 1;
                (pos, value)
            }
            Slot::EndGroup => {
                let (pos, entry) = self.insert_value(mutable, init());
                (pos, entry.take_value().unwrap())
            }
            _ => {
                // not expecting anything else
                panic!("unexpected slot type");
            }
        }
    }

    pub fn replace_value<T: 'static>(&mut self, slot_index: usize, value: T) -> Option<T> {
        assert!(slot_index < self.pos);
        match &mut self.cache.slots[slot_index] {
            Slot::Value(entry) => entry
                .downcast_mut::<T>()
                .expect("entry type mismatch")
                .replace_value(Some(value)),
            _ => {
                panic!("unexpected slot type");
            }
        }
    }*/

    /*pub(crate) fn cache_result<T: Any + Clone>(
        &self,
        key: CallKey,
        input_hash: u64,
        f: impl FnOnce() -> T,
        location: Option<&'static Location<'static>>,
    ) -> T {
        // if an entry already exists and its input hash matches, return it.
        if let Some(entry) = self.entries.borrow().get(&key) {
            match entry.kind {
                CacheEntryKind::FunctionResult {
                    input_hash: entry_input_hash,
                } => {
                    if entry_input_hash == input_hash {
                        return entry
                            .value
                            .downcast_ref::<T>()
                            .expect("cache entry type mismatch")
                            .clone();
                    }
                }
                CacheEntryKind::State => {
                    panic!("unexpected cache entry type")
                }
            }
            assert!(
                entry.input_hash.is_some(),
                "existing cache entry differs in mutability"
            );
            if entry.input_hash == Some(input_hash) && !entry.is_dirty() {}
        }

        let parent = self.dependency_chain.borrow().first().cloned();
        self.dependency_chain.borrow_mut().push(key);
        let value = f();
        self.dependency_chain.borrow_mut().pop();

        match self.entries.borrow_mut().entry(key) {
            Entry::Occupied(mut entry) => {
                // update the existing cache entry with the new value and hash, and reset its dirty
                // flag. Also make sure that the type is correct.
                entry.get_mut().update_function_result(input_hash, value);
                let entry = entry.get_mut();
                entry.replace_value(Some(value));
                entry.input_hash = Some(input_hash);
                entry.dirty.set(false);
                assert_eq!(entry.parent, parent);
            }
            Entry::Vacant(entry) => {
                // insert a fresh entry
                entry.insert(CacheEntry::new_function_result(
                    parent, input_hash, value, location,
                ));
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

    pub(crate) fn cache_state<T: Any + Clone>(
        &self,
        key: CallKey,
        init: impl FnOnce() -> T,
        location: Option<&'static Location<'static>>,
    ) -> T {
        self.cache_impl(key, None, init, location)
    }*/
}

#[derive(Copy, Clone, Debug)]
pub struct CacheInvalidationToken {
    key: GroupKey,
}

struct CacheContext {
    key_stack: CallKeyStack,
    writer: CacheWriter,
}

thread_local! {
    // The cache context is put in TLS so that we don't have to pass an additional parameter
    // to all functions.
    // A less hack-ish solution would be to rewrite composable function calls, but we need
    // more than a proc macro to be able to do that (must resolve function paths and rewrite call sites)
    static CURRENT_CACHE_CONTEXT: RefCell<Option<CacheContext>> = RefCell::new(None);
}

pub struct Cache {
    inner: Option<CacheInner>,
}

impl Cache {
    /// Creates a new cache.
    pub fn new() -> Cache {
        Cache {
            inner: Some(CacheInner::new()),
        }
    }

    /// Runs a cached function with this cache.
    pub fn run<T>(&mut self, function: impl Fn() -> T) -> T {
        CURRENT_CACHE_CONTEXT.with(move |cx_cell| {
            // We can't put a reference type in a TLS.
            // As a workaround, use the classic sleight-of-hand:
            // temporarily move our internals out of self and into the TLS, and move it back to self once we've finished.
            let inner = self.inner.take().unwrap();

            // start writing to the cache
            let writer = CacheWriter::new(inner);

            // initialize the TLS cache context (which contains the cache table writer and the call key stack that maintains
            // unique IDs for each cached function call).
            let cx = CacheContext {
                key_stack: CallKeyStack::new(),
                writer,
            };
            cx_cell.borrow_mut().replace(cx);

            // run the function
            let result = function();

            // finish writing to the cache
            let cx = cx_cell.borrow_mut().take().unwrap();
            // check that calls to CallKeyStack::enter and exit are balanced
            assert!(cx.key_stack.is_empty(), "unbalanced CallKeyStack");

            // finalize cache writer and put the internals back
            self.inner.replace(cx.writer.finish());

            result
        })
    }

    pub fn invalidate(&mut self, token: CacheInvalidationToken) {
        self.inner.as_mut().unwrap().invalidate(token);
    }

    fn with_cx<R>(f: impl FnOnce(&mut CacheContext) -> R) -> R {
        CURRENT_CACHE_CONTEXT.with(|cx_cell| {
            let mut cx = cx_cell.borrow_mut();
            let cx = cx
                .as_mut()
                .expect("function cannot called outside of `Cache::run`");
            f(cx)
        })
    }

    /// Returns the current call identifier.
    pub fn current_call_key() -> CallKey {
        Self::with_cx(|cx| cx.key_stack.current())
    }

    /// Must be called inside `Cache::run`.
    #[track_caller]
    fn enter(index: usize) {
        let location = Location::caller();
        Self::with_cx(move |cx| cx.key_stack.enter(location, index));
    }

    /// Must be called inside `Cache::run`.
    fn exit() {
        Self::with_cx(move |cx| cx.key_stack.exit());
    }

    /// Must be called inside `Cache::run`.
    #[track_caller]
    pub fn scoped<R>(index: usize, f: impl FnOnce() -> R) -> R {
        Self::enter(index);
        let r = f();
        Self::exit();
        r
    }

    /// Returns an invalidation token for the value being calculated.
    pub fn get_invalidation_token() -> CacheInvalidationToken {
        Self::with_cx(move |cx| cx.writer.get_invalidation_token())
    }

    #[track_caller]
    pub fn changed<T: Data>(value: T) -> bool {
        let location = Location::caller();
        Self::with_cx(move |cx| {
            cx.key_stack.enter(location, 0);
            let key = cx.key_stack.current();
            let changed = cx.writer.compare_and_update_value(key, value);
            cx.key_stack.exit();
            changed
        })
    }

    #[track_caller]
    pub fn expect_value<T: Clone + 'static>() -> (Option<T>, usize) {
        let location = Location::caller();
        Self::with_cx(|cx| {
            cx.key_stack.enter(location, 0);
            let key = cx.key_stack.current();
            let r = cx.writer.expect_value::<T>(key);
            cx.key_stack.exit();
            r
        })
    }

    pub fn set_value<T: Clone + 'static>(slot: usize, value: T) {
        Self::with_cx(move |cx| cx.writer.set_value(slot, value))
    }

    #[track_caller]
    pub fn group<R>(f: impl FnOnce(bool) -> R) -> R {
        let location = Location::caller();
        let dirty = Self::with_cx(|cx| {
            cx.key_stack.enter(location, 0);
            cx.writer.start_group(cx.key_stack.current())
        });
        let r = f(dirty);
        Self::with_cx(|cx| {
            cx.writer.end_group();
            cx.key_stack.exit();
        });
        r
    }

    pub fn skip_to_end_of_group() {
        Self::with_cx(|cx| {
            cx.writer.skip_until_end_of_group();
        })
    }

    #[track_caller]
    pub fn memoize<Args: Data, T: Clone + 'static>(args: Args, f: impl FnOnce() -> T) -> T {
        Self::group(move |dirty| {
            let changed = dirty | Self::changed(args);
            let (value, slot) = Self::expect_value::<T>();
            if !changed {
                Self::skip_to_end_of_group();
                value.expect("memoize: no changes in arguments but no value calculated")
            } else {
                let value = f();
                Self::set_value(slot, value.clone());
                value
            }
        })
    }

    #[track_caller]
    pub fn with_state<T: Data, R>(init: impl FnOnce() -> T, update: impl Fn(&mut T) -> R) -> R {
        // load the state from the cache, or reserve a slot if it's the first time we run
        let (mut value, slot) = Self::expect_value::<T>();

        let mut value = if let Some(value) = value {
            // use the existing state
            value
        } else {
            // create the initial value of the state
            init()
        };
        let mut old_value = value.clone();

        let r = update(&mut value);

        // if the state has changed, TODO
        if !old_value.same(&value) {
            Self::set_value(slot, value);
            // TODO: re-run update?
        }

        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_rewrite() {
        let mut cache = CacheInner::new();

        for _ in 0..3 {
            let mut writer = CacheWriter::new(cache);
            writer.start_group(CallKey(99));
            writer.compare_and_update_value(CallKey(1), 0);
            writer.compare_and_update_value(CallKey(2), "hello world".to_string());
            writer.end_group();
            cache = writer.finish();
            cache.dump(0);
        }
    }

    #[test]
    fn test_reorder() {
        use rand::prelude::SliceRandom;

        let mut cache = CacheInner::new();
        let mut rng = rand::thread_rng();
        let mut items = vec![0, 1, 2, 3, 4, 5, 6, 7];

        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            for &item in items.iter() {
                eprintln!(
                    " ==== Iteration {} - item {} =========================",
                    i, item
                );
                writer.start_group(CallKey(item));
                writer.compare_and_update_value(CallKey(100), i);
                writer.end_group();
                writer.dump();
            }
            //writer.dump();
            cache = writer.finish();
            items.shuffle(&mut rng)
        }
    }

    #[test]
    fn test_placeholder() {
        let mut cache = CacheInner::new();

        for _ in 0..3 {
            let mut writer = CacheWriter::new(cache);
            writer.start_group(CallKey(99));
            let changed = writer.compare_and_update_value(CallKey(100), 0);
            let (value, slot) = writer.expect_value::<f64>(CallKey(101));

            if !changed {
                assert!(value.is_some());
                writer.skip_until_end_of_group();
            } else {
                writer.compare_and_update_value(CallKey(102), "hello world".to_string());
                writer.set_value(slot, 0.0);
            }

            writer.end_group();
            cache = writer.finish();
            cache.dump(0);
        }
    }

    #[test]
    fn test_tagged_reorder() {
        use rand::prelude::SliceRandom;

        let mut cache = CacheInner::new();
        let mut rng = rand::thread_rng();
        let mut items = vec![0, 1, 2, 3, 4, 5, 6, 7];

        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            for &item in items.iter() {
                eprintln!(
                    " ==== Iteration {} - item {} =========================",
                    i, item
                );
                writer.compare_and_update_value(CallKey(100 + item), i);
            }
            //writer.dump();
            cache = writer.finish();
            cache.dump(0);
            items.shuffle(&mut rng)
        }
    }

    /*#[test]
    fn test_take_replace() {
        let mut cache = CacheInner::new();
        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            let (index, value) = writer.take_value(false, || 0);
            writer.tagged_compare_and_update_value(CallKey(0), 123);
            writer.dump();
            writer.replace_value(index, i);
            cache = writer.finish();
        }
    }*/

    #[test]
    fn test_insert_remove() {
        use rand::prelude::SliceRandom;

        let mut cache = CacheInner::new();
        let mut rng = rand::thread_rng();

        #[derive(Clone, Debug, Eq, PartialEq)]
        struct Item {
            value: u64,
        }

        impl Data for Item {
            fn same(&self, other: &Self) -> bool {
                self.value == other.value
            }
        }

        impl Item {
            pub fn new(value: u64) -> Item {
                eprintln!("creating Item #{}", value);
                Item { value }
            }
        }

        impl Drop for Item {
            fn drop(&mut self) {
                eprintln!("dropping Item #{}", self.value);
            }
        }

        let mut items = vec![0, 1, 2, 3, 4, 5, 6, 7];

        for i in 0..10 {
            let num_items = rng.gen_range(0..10);
            let items = (0..num_items)
                .map(|_| rng.gen_range(0..10))
                .collect::<Vec<_>>();

            eprintln!("Items: {:?}", items);

            let mut writer = CacheWriter::new(cache);
            for &item in items.iter() {
                //eprintln!(" ==== Iteration {} - item {} =========================", i, item);
                writer.compare_and_update_value(CallKey(item), Item::new(item));
                //writer.dump();
            }
            //writer.dump();
            cache = writer.finish();
        }
    }
}
