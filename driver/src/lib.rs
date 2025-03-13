#![cfg_attr(not(test), no_std)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]
#![feature(generic_const_exprs)]

pub mod actuator;
pub mod bus;
pub mod comm;
pub mod mutex;

#[derive(defmt::Format)]
pub enum Error<C: comm::Comm, M: mutex::Mutex, Output> {
    Mutex(<M as mutex::Mutex>::Error),
    Bus(bus::Error<C, Output>),
}

impl<C: comm::Comm, M: mutex::Mutex, X> Error<C, M, X> {
    #[inline]
    pub fn map<Y, F: FnOnce(X) -> Y>(self, f: F) -> Error<C, M, Y> {
        match self {
            Self::Mutex(e) => Error::Mutex(e),
            Self::Bus(e) => Error::Bus(e.map(f)),
        }
    }
}

/*
impl<C: comm::Comm, M: mutex::Mutex, Output> defmt::Format for Error<C, M, Output> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Mutex(ref e) => {
                write!(f, "Error waiting for permission to use the serial bus: {e}")
            }
            Self::Bus(ref e) => write!(f, "Error using the serial bus: {e}"),
        }
    }
}
*/
