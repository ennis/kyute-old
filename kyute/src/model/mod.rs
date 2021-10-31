mod lens;

/// Trait implemented by types integrated in the view-model system.
// TODO once specialization lands (lol), we could replace this with a blanket impl that defaults
// to `()` (i.e. "something changed, but the type doesn't provide more precise information, so assume everything has changed")
pub trait Model {
    /// Describes an incremental change to this data model.
    type Change;
}

impl<'a, T: Model> Model for &'a T {
    type Change = T::Change;
}

impl<'a, T: Model> Model for &'a mut T {
    type Change = T::Change;
}

macro_rules! impl_model_simple {
    ($t:ty) => {
        impl Model for $t {
            type Change = $t;
        }
    };
}

impl_model_simple!(());
impl_model_simple!(u8);
impl_model_simple!(u16);
impl_model_simple!(u32);
impl_model_simple!(u64);
impl_model_simple!(u128);
impl_model_simple!(i8);
impl_model_simple!(i16);
impl_model_simple!(i32);
impl_model_simple!(i64);
impl_model_simple!(i128);
impl_model_simple!(f32);
impl_model_simple!(f64);
impl_model_simple!(char);
impl_model_simple!(String);

/// Represents a change to a collection.
#[derive(Copy, Clone)]
pub enum CollectionChange {
    Insert { index: usize },
    Remove { index: usize },
    Move { from: usize, to: usize },
}

impl<T: Model> Model for Vec<T> {
    type Change = CollectionChange;
}
