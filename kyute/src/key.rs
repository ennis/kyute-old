use std::panic::Location;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Key {
    caller_location: &'static Location<'static>,
}

impl Key {
    #[track_caller]
    pub fn from_caller() -> Key {
        Key {
            caller_location: Location::caller().into()
        }
    }
}