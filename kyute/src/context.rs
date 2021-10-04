use crate::{
    cache::Cache,
    call_key::{CallKey, CallKeyStack},
    data::Data,
};
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
use crate::cache::{ValueEntryKey, CacheWriter};

/*enum ContextCacheState {
    Idle(Cache),
    Writing(CacheWriter),
}

impl ContextCacheState {
    fn writer(&mut self) -> &mut CacheWriter {
        match self {
            ContextState::Idle(_) => {
                panic!("not recomposing")
            }
            ContextState::Writing(writer) => writer
        }
    }
}
*/

struct ContextImpl {
    key_stack: CallKeyStack,
   // cache: ContextCacheState,
}

#[doc(hidden)]
pub struct StateCell<T> {
    slot_index: usize,
    value_key: ValueEntryKey,
    changed: bool,
    value: T
}

impl<T> Drop for StateCell<T> {
    fn drop(&mut self) {
        todo!()
    }
}

impl ContextImpl {
    fn enter_scope(&mut self, location: &'static Location<'static>, index: usize) -> CallKey {
        self.key_stack.enter(location, index)
    }

    fn exit_scope(&mut self) {
        self.key_stack.exit();
    }

    /*pub fn state<T>(&mut self, location: &'static Location<'static>, init: impl FnOnce() -> T) -> StateCell<T>
        where
            T: Any + Clone,
    {
        let call_key = self.enter_scope(location, 0);
        let (index, entry) = self.cache.writer().tagged_take_value(call_key, true, init);
        let state_cell = StateCell {
            slot_index: index,
            value_key: entry.key.unwrap(),
            changed: false,
            value: ()
        };
        self.exit_scope();
        state_cell
    }*/

}

#[derive(Clone)]
pub struct Context(Arc<RefCell<ContextImpl>>);

thread_local! {
    pub static CONTEXT: Context = Context::new();
}

impl Context {
    fn new() -> Context {
        Context(Arc::new(RefCell::new(ContextImpl {
            key_stack: CallKeyStack::new()
            //cache: RefCell::new(Cache::new()),
        })))
    }

    /*fn set_state<T: 'static>(&self, key: CallKey, val: T) {
        self.0.cache.set_state(key, val);
    }*/
}


impl Context {
    pub fn current() -> Context {
        CONTEXT.with(|x| x.clone())
    }

    pub fn current_call_key() -> CallKey {
        CONTEXT.with(|cx| cx.0.borrow().key_stack.current())
    }

    #[track_caller]
    pub fn enter(index: usize) -> CallKey {
        let location = Location::caller();
        CONTEXT.with(|cx| cx.0.borrow_mut().enter_scope(location, index))
    }

    pub fn exit() {
        CONTEXT.with(|cx| cx.0.borrow_mut().exit_scope())
    }



    /// Records a mutable state entry in the cache.
    #[track_caller]
    pub fn state<T>(init: impl FnOnce() -> T) -> StateCell<T>
    where
        T: Any + Clone,
    {
        let location = Location::caller();
        todo!()
    }

    #[track_caller]
    pub fn scoped<R>(index: usize, f: impl FnOnce() -> R) -> R {
        Context::enter(0);
        let r = f();
        Context::exit();
        r
    }

    pub fn dump() {
       //CONTEXT.with(|cx| cx.0.cache.dump());
    }
}

impl CallKey {
    pub fn current() -> CallKey {
        Context::current_call_key()
    }
}
