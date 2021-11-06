use crate::{Cache, cache::CacheInner, call_key::{CallKey, CallKeyStack}, data::Data};
use std::{
    any::Any,
    cell::{Ref, RefCell},
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    panic::Location,
    sync::Arc,
};
use std::ops::Deref;
use crate::cache::CacheWriter;


impl CallKey {
    pub fn current() -> CallKey {
        Cache::current_call_key()
    }
}
