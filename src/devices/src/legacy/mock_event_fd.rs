use std::ops::Deref;
use vm_superio::Trigger;

use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::{io, result};

// Reexport commonly used flags from libc.
pub use libc::{EFD_CLOEXEC, EFD_NONBLOCK, EFD_SEMAPHORE};

#[derive(Clone)]
pub struct EventFd {
}

impl EventFd {

    pub fn new(flag: i32) -> result::Result<EventFd, io::Error> {
        unimplemented!()
    }


    pub fn write(&self, v: u64) -> result::Result<(), io::Error> {
        unimplemented!()

    }


    pub fn read(&self) -> result::Result<u64, io::Error> {
        unimplemented!()

    }

    pub fn try_clone(&self) -> result::Result<EventFd, io::Error> {
        unimplemented!()

    }
}

impl AsRawFd for EventFd {
    fn as_raw_fd(&self) -> RawFd {
        unimplemented!()
    }
}

impl FromRawFd for EventFd {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        unimplemented!()
    }
}

/// Newtype for implementing the trigger functionality for `EventFd`.
///
/// The trigger is used for handling events in the legacy devices.
pub struct EventFdTrigger(EventFd);

impl Trigger for EventFdTrigger {
    type E = io::Error;

    fn trigger(&self) -> io::Result<()> {
        unimplemented!()
    }
}
impl Deref for EventFdTrigger {
    type Target = EventFd;
    fn deref(&self) -> &Self::Target {
        unimplemented!()
    }
}
impl EventFdTrigger {
    fn try_clone(&self) -> io::Result<Self> {
        unimplemented!()
    }
    fn new(evt: EventFd) -> Self {
        unimplemented!()
    }

    pub fn get_event(&self) -> EventFd {
        unimplemented!()
    }
}