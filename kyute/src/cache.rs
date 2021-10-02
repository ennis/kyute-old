use crate::{call_key::CallKey, data::Data};
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
    pub struct CacheEntryKey;
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

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct CacheEntryIndex(usize);

/// Internal cache entry.
struct CacheEntry<T: ?Sized> {
    /// Relative position of the parent entry, or 0 if this is a root entry.
    parent: i32,
    occupied: bool,
    type_id: std::any::TypeId,
    location: Option<&'static Location<'static>>,
    value: ManuallyDrop<T>,
}

impl<T: Any> CacheEntry<T> {
    /// Creates a new cache entry.
    fn new(
        parent: i32,
        initial_value: T,
        location: Option<&'static Location<'static>>,
    ) -> CacheEntry<T> {
        CacheEntry {
            parent,
            occupied: true,
            type_id: std::any::TypeId::of::<T>(),
            location,
            value: ManuallyDrop::new(initial_value),
        }
    }

    fn get(&self) -> Option<&T> {
        if self.occupied {
            Some(&self.value)
        } else {
            None
        }
    }

    fn get_mut(&mut self) -> Option<&mut T> {
        if self.occupied {
            Some(&mut self.value)
        } else {
            None
        }
    }

    /*fn update(&mut self, parent: Option<CacheEntryIndex>, value: T) {
        assert_eq!(self.parent, parent);
        self.replace_value(Some(value));
    }*/

    /// Extracts the value of this entry.
    fn take_value(&mut self) -> Option<T> {
        self.replace_value(None)
    }

    /// Replaces the value of this entry.
    fn replace_value(&mut self, value: Option<T>) -> Option<T> {
        // extract old value
        let old_value = if self.occupied {
            self.occupied = false;
            unsafe { Some(ManuallyDrop::take(&mut self.value)) }
        } else {
            None
        };

        // replace value
        if let Some(value) = value {
            unsafe {
                *(&mut self.value as *mut ManuallyDrop<dyn Any> as *mut ManuallyDrop<T>) =
                    ManuallyDrop::new(value);
                self.occupied = true;
            }
        }

        old_value
    }
}

impl CacheEntry<dyn Any> {
    pub fn downcast<T: Any>(&self) -> Option<&CacheEntry<T>> {
        if self.type_id == std::any::TypeId::of::<T>() {
            unsafe { Some(&*(self as *const CacheEntry<dyn Any> as *const CacheEntry<T>)) }
        } else {
            None
        }
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut CacheEntry<T>> {
        if self.type_id == std::any::TypeId::of::<T>() {
            unsafe { Some(&mut *(self as *mut CacheEntry<dyn Any> as *mut CacheEntry<T>)) }
        } else {
            None
        }
    }
}

impl<T: ?Sized> Drop for CacheEntry<T> {
    fn drop(&mut self) {
        if self.occupied {
            self.occupied = false;
            unsafe {
                ManuallyDrop::drop(&mut self.value);
            }
        }
    }
}

enum Slot {
    Tag(CallKey),
    /// Marks the start of a group.
    /// Contains the length of the group including this slot and the `GroupEnd` marker.
    StartGroup {
        len: u32,
        parent: i32,
        dirty: bool,
    },
    /// Marks the end of a scope.
    EndGroup,
    /// Holds a piece of state.
    State(Box<CacheEntry<dyn Any>>),
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

/*pub struct Cache {
    /// Vector of cache entries. They follow the order in which they are created during composition.
    slots: Vec<Slot>,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            slots: RefCell::new(Default::default()),
        }
    }

    /// Recursively marks this entry and its parents as dirty.
    fn dirty_entry_recursive(&self, key: CallKey) {
        let entries = self.slots.borrow();
        let entry = entries.get(&key).unwrap();
        entry.dirty.set(true);
        if let Some(parent) = entry.parent {
            self.dirty_entry_recursive(parent)
        }
    }

    pub(crate) fn dump(&self) {
        eprintln!("====== Cache entries: ======");
        for (key, entry) in self.slots.borrow().iter() {
            if let Some(location) = entry.location {
                eprintln!("- [{}]({:?}) - parent: {:?}", location, key, entry.parent);
            } else {
                eprintln!("- {:?} - parent: {:?}", key, entry.parent);
            }
        }
    }

    pub(crate) fn take_state<T: Any>(&self, key: CallKey) -> Result<Option<T>, CacheEntryError> {
        let mut entries = self.slots.borrow_mut();
        let mut entry = entries
            .get_mut(&key)
            .ok_or(CacheEntryError::EntryNotFound)?;
        entry.take_value()
    }

    /// Replaces the value of a mutable state entry.
    pub(crate) fn replace_state<T: Any>(
        &self,
        key: CallKey,
        value: Option<T>,
    ) -> Result<Option<T>, CacheEntryError> {
        let mut entries = self.slots.borrow_mut();
        let mut entry = entries
            .get_mut(&key)
            .ok_or(CacheEntryError::EntryNotFound)?;
        let result = entry.replace_value(value);
        if matches!(result, Ok(Some(_))) {
            tracing::warn!("cache entry already contained a value");
        }
        result
    }

    pub fn into_writer(self) -> CacheWriter {
        CacheWriter {
            cache: self,
            pos: 0,
            group_start: None,
            return_pos: vec![],
        }
    }
}*/

pub struct Cache {
    slots: Vec<Slot>,
    revision: usize,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            slots: vec![
                Slot::StartGroup {
                    len: 2,
                    parent: 0,
                    dirty: false,
                },
                Slot::EndGroup,
            ],
            revision: 0,
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
                Slot::Tag(call_key) => {
                    eprintln!("{:3} Tag        {:?}", i, call_key)
                }
                Slot::StartGroup { len, dirty, parent } => {
                    eprintln!(
                        "{:3} StartGroup len={} (end={}) parent={} {}",
                        i,
                        *len,
                        i + *len as usize - 1,
                        parent,
                        if *dirty { "dirty" } else { "" }
                    )
                }
                Slot::EndGroup => {
                    eprintln!("{:3} EndGroup", i)
                }
                Slot::State(entry) => {
                    eprintln!(
                        "{:3} State      {:?} parent={} {}",
                        i,
                        entry.type_id,
                        entry.parent,
                        if entry.occupied { "" } else { "vacant" }
                    )
                }
            }
        }
    }
}

/// Used to update a cache in a composition context.
pub struct CacheWriter {
    /// The cache being updated
    cache: Cache,
    /// Current writing position
    pos: usize,
    /// Start of the current group
    group_start: Option<usize>,
    /// return index
    group_stack: Vec<usize>,
}

impl CacheWriter {
    pub fn new(cache: Cache) -> CacheWriter {
        let mut writer = CacheWriter {
            cache,
            pos: 0,
            group_start: None,
            group_stack: vec![],
        };
        writer.start_untagged_group();
        writer
    }

    /// Finishes writing to the cache, returns the updated cache object.
    pub fn finish(mut self) -> Cache {
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
                Slot::Tag(key) if call_key == key => return Some(i),
                Slot::StartGroup { len, .. } => {
                    i += len as usize;
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
                self.pos += 1;
                true
            }
            None => {
                // insert tag
                self.cache.slots.insert(self.pos, Slot::Tag(call_key));
                self.pos += 1;
                false
            }
        }
    }

    fn parent_group_offset(&self) -> i32 {
        if let Some(&parent) = self.group_stack.last() {
            parent as i32 - self.pos as i32
        } else {
            0
        }
    }

    pub fn start_group(&mut self, call_key: CallKey) {
        let tag_found = self.sync(call_key);

        let parent = self.parent_group_offset();
        if tag_found {
            match &mut self.cache.slots[self.pos] {
                Slot::StartGroup {
                    parent: old_parent, ..
                } => {
                    *old_parent = parent;
                }
                _ => panic!("unexepected slot type"),
            }
        } else {
            // insert new group - start and end markers
            self.cache.slots.insert(
                self.pos,
                Slot::StartGroup {
                    len: 2,
                    parent,
                    dirty: false,
                },
            ); // 2 = initial length of group (start+end slots)
            self.cache.slots.insert(self.pos + 1, Slot::EndGroup);
        }

        // enter group
        self.group_stack.push(self.pos);
        self.pos += 1;
    }

    pub fn start_untagged_group(&mut self) {
        let parent = self.parent_group_offset();
        match &mut self.cache.slots[self.pos] {
            Slot::EndGroup => {
                // end of current group: insert new group tags
                self.cache.slots.insert(
                    self.pos,
                    Slot::StartGroup {
                        len: 2, // initial length of group (start+end slots)
                        parent,
                        dirty: false,
                    },
                );
                self.cache.slots.insert(self.pos + 1, Slot::EndGroup);
                // enter group
                self.group_stack.push(self.pos);
                self.pos += 1;
            }
            Slot::StartGroup {
                parent: old_parent, ..
            } => {
                *old_parent = parent;
                // enter group
                self.group_stack.push(self.pos);
                self.pos += 1;
            }
            _ => {
                // inserting an untagged group: either the next element is the group we expect,
                // or we reached the end of the current group because it's the first time we're
                // opening the untagged group.
                panic!("expected GroupStart or end of current group")
            }
        }
    }

    pub fn dump(&self) {
        eprintln!("position : {}", self.pos);
        eprintln!("stack    : {:?}", self.group_stack);
        eprintln!("slots:");
        self.cache.dump(self.pos);
    }

    fn group_end_position(&self) -> usize {
        let mut level = 0;
        for i in self.pos..self.cache.slots.len() {
            match &self.cache.slots[i] {
                Slot::StartGroup { .. } => {
                    level += 1;
                }
                Slot::EndGroup => {
                    if level == 0 {
                        return i;
                    } else {
                        level -= 1;
                    }
                }
                _ => {}
            }
        }
        panic!("could not find matching EndGroup");
    }

    pub fn end_group(&mut self) {
        // all remaining slots in the group are now considered dead in this revision:
        // - find position of group end marker
        let group_end_pos = self.group_end_position();
        // - remove the extra nodes
        self.cache.slots.drain(self.pos..group_end_pos);
        // skip GroupEnd marker
        self.pos += 1;
        // update group length
        let group_start_pos = self.group_stack.pop().expect("unbalanced groups");
        self.cache.slots[group_start_pos].update_group_len(self.pos - group_start_pos);
    }

    pub fn skip(&mut self) {
        loop {
            let parent = self.parent_group_offset();
            match &mut self.cache.slots[self.pos] {
                Slot::StartGroup {
                    parent: old_parent,
                    len,
                    ..
                } => {
                    *old_parent = parent;
                    self.pos += *len as usize;
                    break;
                }
                Slot::EndGroup => {
                    panic!("unexpected EndGroup in skip")
                }
                Slot::Tag(_) => self.pos += 1,
                Slot::State(_) => {
                    self.pos += 1;
                    break;
                }
            }
        }
    }

    fn insert_value<T: 'static>(&mut self, value: T) -> usize {
        let pos = self.pos;
        self.cache.slots.insert(
            self.pos,
            Slot::State(Box::new(CacheEntry::new(
                self.parent_group_offset(),
                value,
                None,
            ))),
        );
        self.pos += 1;
        pos
    }

    fn insert_value_and_take<T: Any>(&mut self, value: T) -> (usize, T) {
        let pos = self.pos;
        let mut entry = CacheEntry::new(self.parent_group_offset(), value, None);
        let value = entry.take_value().unwrap();
        self.cache
            .slots
            .insert(self.pos, Slot::State(Box::new(entry)));
        self.pos += 1;
        (pos, value)
    }

    pub fn tagged_compare_and_update_value<T: Data>(
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
    }

    pub fn compare_and_update_value<T: Data>(&mut self, new_value: T) -> bool {
        let parent = self.parent_group_offset();
        match &mut self.cache.slots[self.pos] {
            Slot::State(entry) => {
                entry.parent = parent;
                let entry = entry.downcast_mut::<T>().expect("entry type mismatch");
                let value = entry.get_mut().expect("expected occupied entry");
                self.pos += 1;
                if !new_value.same(&value) {
                    *value = new_value;
                    true
                } else {
                    false
                }
            }
            Slot::EndGroup => {
                // insert entry
                self.insert_value(new_value);
                true
            }
            _ => {
                // not expecting anything else
                panic!("unexpected slot type");
            }
        }
    }

    pub fn tagged_take_value<T: 'static>(
        &mut self,
        call_key: CallKey,
        init: impl FnOnce() -> T,
    ) -> (usize, T) {
        if self.sync(call_key) {
            self.take_value(init)
        } else {
            self.insert_value_and_take(init())
        }
    }

    pub fn take_value<T: 'static>(&mut self, init: impl FnOnce() -> T) -> (usize, T) {
        let parent = self.parent_group_offset();
        match &mut self.cache.slots[self.pos] {
            Slot::State(entry) => {
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
            Slot::EndGroup => self.insert_value_and_take(init()),
            _ => {
                // not expecting anything else
                panic!("unexpected slot type");
            }
        }
    }

    pub fn replace_value<T: 'static>(&mut self, slot_index: usize, value: T) -> Option<T> {
        assert!(slot_index < self.pos);
        match &mut self.cache.slots[slot_index] {
            Slot::State(entry) => entry
                .downcast_mut::<T>()
                .expect("entry type mismatch")
                .replace_value(Some(value)),
            _ => {
                panic!("unexpected slot type");
            }
        }
    }

    // can't do that safely
    /*pub fn skip_until_end_of_group(&mut self) {
        loop {
            if let Slot::EndGroup = self.cache.slots[self.pos] {
                break;
            }
            self.pos += 1;
            continue;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_rewrite() {
        let mut cache = Cache::new();

        for _ in 0..3 {
            let mut writer = CacheWriter::new(cache);
            writer.start_untagged_group();
            writer.compare_and_update_value(0);
            writer.compare_and_update_value("hello world".to_string());
            writer.end_group();
            cache = writer.finish();
        }
    }

    #[test]
    fn test_reorder() {
        use rand::prelude::SliceRandom;

        let mut cache = Cache::new();
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
                writer.compare_and_update_value(i);
                writer.end_group();
                writer.dump();
            }
            //writer.dump();
            cache = writer.finish();
            items.shuffle(&mut rng)
        }
    }

    #[test]
    fn test_tagged_reorder() {
        use rand::prelude::SliceRandom;

        let mut cache = Cache::new();
        let mut rng = rand::thread_rng();
        let mut items = vec![0, 1, 2, 3, 4, 5, 6, 7];

        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            for &item in items.iter() {
                eprintln!(
                    " ==== Iteration {} - item {} =========================",
                    i, item
                );
                writer.tagged_compare_and_update_value(CallKey(item), i);
                writer.dump();
            }
            //writer.dump();
            cache = writer.finish();
            items.shuffle(&mut rng)
        }
    }

    #[test]
    fn test_take_replace() {
        let mut cache = Cache::new();
        for i in 0..3 {
            let mut writer = CacheWriter::new(cache);
            let (index, value) = writer.take_value(|| 0);
            writer.tagged_compare_and_update_value(CallKey(0), 123);
            writer.dump();
            writer.replace_value(index, i);
            cache = writer.finish();
        }
    }

    #[test]
    fn test_insert_remove() {
        use rand::prelude::SliceRandom;

        let mut cache = Cache::new();
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
                writer.tagged_compare_and_update_value(CallKey(item), Item::new(item));
                //writer.dump();
            }
            //writer.dump();
            cache = writer.finish();
        }
    }
}
