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

struct ContextImpl {
    key_stack: RefCell<CallKeyStack>,
    cache: Cache,
}

impl ContextImpl {
    fn enter_scope(&self, location: &'static Location<'static>, index: usize) -> CallKey {
        self.key_stack.borrow_mut().enter(location, index)
    }

    fn exit_scope(&self) {
        self.key_stack.borrow_mut().exit();
    }

    /*// TODO: replace with an explicit dependency on the
    fn cache<T, Args>(
        &self,
        location: &'static Location<'static>,
        args: Args,
        f: impl FnOnce(&Args) -> T,
    ) -> (CallKey, T)
    where
        T: Any + Clone,
        Args: Hash,
    {
        let key = self.enter_scope(location, 0);
        let val = self.cache.cache(key, args, f, Some(location));
        self.exit_scope();
        (key, val)
    }

    fn cache_state<T>(
        &self,
        location: &'static Location<'static>,
        f: impl FnOnce() -> T,
    ) -> (CallKey, T)
        where
            T: Any + Clone,
    {
        let key = self.enter_scope(location, 0);
        let val = self.cache.cache_state(key, f, Some(location));
        self.exit_scope();
        (key, val)
    }*/
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
            cache: Cache::new(),
        }))
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

    /*#[track_caller]
    pub fn cache<T, Args>(args: Args, f: impl FnOnce(&Args) -> T) -> T
    where
        T: Any + Clone,
        Args: Hash,
    {
        let location = Location::caller();
        CONTEXT.with(|cx| cx.0.cache(location, args, f).1)
    }

    #[track_caller]
    pub fn state<T>(init: impl FnOnce() -> T) -> State<T>
    where
        T: Any + Clone,
    {
        let location = Location::caller();
        CONTEXT.with(|cx| {
            let (key, value) = cx.0.cache_state(location, init);
            State {
                context: cx.clone(),
                key,
                value,
                _phantom: Default::default(),
            }
        })
    }*/

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
