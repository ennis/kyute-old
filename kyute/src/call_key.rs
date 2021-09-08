use std::{
    collections::hash_map::DefaultHasher,
    fmt,
    fmt::Formatter,
    hash::{Hash, Hasher},
    panic::Location,
};

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CallKey(u64);

impl fmt::Debug for CallKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CallKey")
            .field(&format_args!("{:016X}", self.0))
            .finish()
    }
}

/// The ID stack. Each level corresponds to a parent ItemNode.
pub(crate) struct CallKeyStack(Vec<u64>);

impl CallKeyStack {
    /// Creates a new IdStack.
    pub(crate) fn new() -> CallKeyStack {
        CallKeyStack(vec![])
    }

    fn chain_hash<H: Hash>(&self, s: &H) -> u64 {
        let stacklen = self.0.len();
        let key1 = if stacklen >= 2 {
            self.0[stacklen - 2]
        } else {
            0
        };
        let key0 = if stacklen >= 1 {
            self.0[stacklen - 1]
        } else {
            0
        };
        let mut hasher = DefaultHasher::new();
        key0.hash(&mut hasher);
        key1.hash(&mut hasher);
        s.hash(&mut hasher);
        hasher.finish()
    }

    pub(crate) fn enter(&mut self, location: &Location, index: usize) -> CallKey {
        let key = self.chain_hash(&(location, index));
        self.0.push(key);
        CallKey(key)
    }

    pub(crate) fn exit(&mut self) {
        self.0.pop();
    }

    pub(crate) fn current(&self) -> CallKey {
        CallKey(*self.0.last().unwrap())
    }
}
