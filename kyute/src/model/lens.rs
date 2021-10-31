/// Trait implemented by "lens" types, which act like a reified accessor for
/// some part of type U of an object of type T.
pub trait Lens<T, U> {
    /// Returns a reference to the object part that the lens is watching.
    fn with<'a, R>(&self, data: &'a T, f: impl FnOnce(&U) -> R) -> R;

    /// Returns a mutable reference to the object part that the lens is watching.
    fn with_mut<'a, R>(&self, data: &'a mut T, f: impl FnOnce(&mut U) -> R) -> R;
}

macro_rules! impl_field_lens {
    ($name:ident,$T:ty;$U:ty;$field:ident) => {
        impl<$T, $U> Lens<$T, $U> for $name {
            fn with<'a, R>(&self, data: &'a $T, f: impl FnOnce(&$U) -> R) -> R {
                f(&data.$field)
            }

            fn with_mut<'a, R>(&self, data: &'a mut $T, f: impl FnOnce(&mut $U) -> R) -> R {
                f(&mut data.$field)
            }
        }
    };
}

/*// augmentation
impl<T,U> Lens<T,(T,U)> for T {
    fn with<'a, R>(&self, data: &'a T, f: impl FnOnce(&(T,U)) -> R) -> R {
        todo!()
    }

    fn with_mut<'a, R>(&self, data: &'a mut T, f: impl FnOnce(&mut (T,U)) -> R) -> R {
        todo!()
    }
}*/