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

pub enum IoError<C: comm::Comm> {
    Send(<C as comm::Comm>::SendError),
    Recv(<C as comm::Comm>::RecvError),
}

impl<C: comm::Comm> defmt::Format for IoError<C> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Send(ref e) => defmt::write!(f, "Error while sending a packet: {}", e),
            Self::Recv(ref e) => defmt::write!(f, "Error while receiving a packet: {}", e),
        }
    }
}

pub enum BusError<C: comm::Comm, M: mutex::Mutex, Output> {
    Mutex(<M as mutex::Mutex>::Error),
    Packet(bus::Error<C, Output>),
}

impl<C: comm::Comm, M: mutex::Mutex, Output> defmt::Format for BusError<C, M, Output> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Mutex(ref e) => defmt::write!(
                f,
                "Mutex error while waiting to use the Dynamixel serial bus: {}",
                e
            ),
            Self::Packet(ref e) => defmt::write!(f, "Error from the Dynamixel serial bus: {}", e),
        }
    }
}

pub enum ActuatorError<C: comm::Comm, M: mutex::Mutex> {
    Mutex(<M as mutex::Mutex>::Error),
    Packet(actuator::Error<C>),
}

impl<C: comm::Comm, M: mutex::Mutex> defmt::Format for ActuatorError<C, M> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Mutex(ref e) => defmt::write!(
                f,
                "Mutex error while waiting to use the Dynamixel serial bus: {}",
                e,
            ),
            Self::Packet(ref e) => defmt::write!(f, "Error from the Dynamixel serial bus: {}", e),
        }
    }
}
