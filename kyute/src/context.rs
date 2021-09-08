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
    mem,
    panic::Location,
    sync::Arc,
};

struct ContextImpl {
    key_stack: RefCell<CallKeyStack>,
    cache_entry_stack: RefCell<Vec<CallKey>>,
    cache: Cache,
}

impl ContextImpl {
    fn enter_scope(&self, location: &'static Location<'static>, index: usize) -> CallKey {
        self.key_stack.borrow_mut().enter(location, index)
    }

    fn exit_scope(&self) {
        self.key_stack.borrow_mut().exit();
    }

    fn cache<T, Args>(
        &self,
        location: &'static Location<'static>,
        args: Args,
        f: impl FnOnce(&Args) -> T,
    ) -> T
    where
        T: Any + Data,
        Args: Hash,
    {
        let key = self.enter_scope(location, 0);
        let r = self.cache.cache(key, args, f, Some(location));
        self.exit_scope();
        r
    }
}

#[derive(Clone)]
pub struct Context(Arc<ContextImpl>);

thread_local! {
    pub static CONTEXT: Context = Context::new();
}

impl Context {
    fn new() -> Context {
        Context(Arc::new(ContextImpl {
            key_stack: RefCell::new(CallKeyStack::new()),
            cache_entry_stack: RefCell::new(vec![]),
            cache: Cache::new(),
        }))
    }

    pub fn current() -> Context {
        CONTEXT.with(|x| x.clone())
    }

    pub fn current_call_key() -> CallKey {
        CONTEXT.with(|cx| cx.0.key_stack.borrow().current())
    }

    #[track_caller]
    pub fn enter(index: usize) -> CallKey {
        let location = Location::caller();
        CONTEXT.with(|cx| cx.0.enter_scope(location, index))
    }

    pub fn exit() {
        CONTEXT.with(|cx| cx.0.exit_scope())
    }

    #[track_caller]
    pub fn cache<T, Args>(args: Args, f: impl FnOnce(&Args) -> T) -> T
    where
        T: Any + Data,
        Args: Hash,
    {
        let location = Location::caller();
        CONTEXT.with(|cx| cx.0.cache(location, args, f))
    }

    #[track_caller]
    pub fn in_scope<R>(index: usize, f: impl FnOnce() -> R) -> R {
        Context::enter(0);
        let r = f();
        Context::exit();
        r
    }

    pub fn dump() {
        CONTEXT.with(|cx| cx.0.cache.dump());
    }
}

impl CallKey {
    pub fn current() -> CallKey {
        Context::current_call_key()
    }
}
