use std::{fmt, panic::Location};

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash, Debug)]
pub struct Key {
    caller_location: &'static Location<'static>,
    id: u64,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}+{}", self.caller_location, self.id)
    }
}

impl Key {
    #[track_caller]
    pub fn from_caller(id: u64) -> Key {
        Key {
            caller_location: Location::caller().into(),
            id,
        }
    }

    pub fn caller_location(&self) -> &'static Location<'static> {
        self.caller_location
    }
}
