use std::cell::RefCell;
use crate::Data;

pub struct CacheCell<Args,T> {
    last: RefCell<Option<(Args,T)>>
}

impl<Args,T> CacheCell<Args,T> {
    pub fn new() -> CacheCell<Args,T> {
        CacheCell {
            last: RefCell::new(None),
        }
    }
}

impl<Args,T> Default for CacheCell<Args,T> {
    fn default() -> Self {
        CacheCell::new()
    }
}

impl<Args: Data, T: Clone> CacheCell<Args,T> {
    pub fn cache(&self, args: Args, init: impl FnOnce(&Args) -> T) -> T {
        let mut last = self.last.borrow_mut();
        if let Some((last_args,last_value)) = &*last {
            if last_args.same(&args) {
                return last_value.clone();
            }
        }

        let value = init(&args);
        *last = Some((args, value.clone()));
        value
    }
}
