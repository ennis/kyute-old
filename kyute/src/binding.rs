use crate::{model::CollectionChange, Model};
use std::borrow::Cow;

pub trait Lens<T: Model, U: Clone> {
    fn get(&self, data: &T) -> Cow<U>;
    fn set(&self, data: &mut T, value: U);
    fn affected(&self, change: &T::Change) -> bool;
}

// Allowable signatures:
// - `Fn() -> Vec<Item>`   (constant, unaffected by change)
// - `Fn(&T) -> Vec<Item>` (non-incremental)
// - `Fn(&T, &T::Change) -> CollectionUpdate`
pub trait ListBinding<T: Model, Item> {}

//--------------------------------------------------------------------------------------------------
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct ValueLens<U: Clone>(U);

impl<U: Clone> ValueLens<U> {
    pub fn new(value: U) -> ValueLens<U> {
        ValueLens(value)
    }
}

impl<T: Model, U: Clone> Lens<T, U> for ValueLens<U> {
    fn get(&self, data: &T) -> Cow<U> {
        Cow::Borrowed(&self.0)
    }

    fn set(&self, data: &mut T, value: U) {
        unimplemented!()
    }

    fn affected(&self, change: &T::Change) -> bool {
        false
    }
}

impl<U: Clone> From<U> for ValueLens<U> {
    fn from(value: U) -> Self {
        ValueLens::new(value)
    }
}

// F can be zero-sized
impl<T, F, U> Lens<T, U> for F
where
    T: Model,
    U: Clone,
    F: Fn() -> U,
{
    fn get(&self, _data: &T) -> Cow<U> {
        Cow::Owned((self)())
    }

    fn set(&self, _data: &mut T, value: U) {
        unimplemented!()
    }

    fn affected(&self, change: &T::Change) -> bool {
        false
    }
}

pub trait LensExt<T: Model, U: Clone>: Lens<T, U> {
    fn get_owned(&self, data: &T) -> U::Owned
    where
        U: ToOwned,
    {
        self.get(data).into_owned()
    }
}

impl<L, T, U> LensExt<T, U> for L
where
    T: Model,
    U: Clone,
    L: Lens<T, U>,
{
}

/*/// Evaluates to a zero-sized lens that always returns the given value.
macro_rules! constant_lens {
    ($value:expr) => {
        struct ConstantLens;
        impl<T: $crate::Model, $u> $crate::Lens for ConstantLens {
            fn get(&self, data: &T) -> Cow<$u> {
                Cow::Borrowed($value)
            }
            fn set(&self, data: &mut T, value: $u) {
                unimplemented!()
            }
            fn affected(&self, change: &T::Change) -> bool {
                false
            }
        }
        ConstantLens
    };
}

pub(crate) use constant_lens;*/

// no allocation if zero-sized
// -> however, this means that
pub type DynLens<T, U> = Box<dyn Lens<T, U>>;

impl<T: Model, U: Clone> Lens<T, U> for Box<dyn Lens<T, U>> {
    fn get(&self, data: &T) -> Cow<U> {
        (**self).get(data)
    }

    fn set(&self, data: &mut T, value: U) {
        (**self).set(data, value)
    }

    fn affected(&self, change: &T::Change) -> bool {
        (**self).affected(change)
    }
}

//--------------------------------------------------------------------------------------------------

/*/// A modification to an indexed collection.
pub enum CollectionUpdate<Item> {
    InsertOne {
        index: usize,
        item: Item,
    },
    InsertSlice {
        start: usize,
        len: usize,
        items: Vec<Item>,
    },
    RemoveOne {
        index: usize,
    },
    RemoveSlice {
        start: usize,
        len: usize,
    },
    Replace {
        index: usize,
        item: Item,
    },
    ReplaceAll {
        items: Vec<Item>,
    },
}*/

/*pub struct BoundVec<T: Model, Item> {
    items: Vec<Item>,
    f: Box<dyn Fn(&T, &T::Change, &mut Vec<Item>) -> Option<CollectionChange>>,
}

impl<T: Model, Item> BoundVec<T, Item> {
    pub fn new() -> BoundVec<T, Item> {
        BoundVec {
            items: vec![],
            f: Box::new(|_, _, _| None),
        }
    }

    pub fn bind(&mut self, f: impl Fn(&T, &T::Change, &mut Vec<Item>) -> Option<CollectionChange>) {
        self.items.clear();
        self.f = Box::new(f);
    }

    pub fn update(&mut self, data: &T, change: &T::Change) -> Option<CollectionChange> {
        (self.f)(data, change, &mut self.items)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Item> {
        self.items.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}
*/