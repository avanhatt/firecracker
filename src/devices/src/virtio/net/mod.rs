// Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::{io, result};

pub const MAX_BUFFER_SIZE: usize = 65562;
pub const QUEUE_SIZE: u16 = 256;
pub const NUM_QUEUES: usize = 2;
pub const QUEUE_SIZES: &[u16] = &[QUEUE_SIZE; NUM_QUEUES];
// The index of the rx queue from Net device queues/queues_evts vector.
pub const RX_INDEX: usize = 0;
// The index of the tx queue from Net device queues/queues_evts vector.
pub const TX_INDEX: usize = 1;

pub mod device;
pub mod event_handler;
pub mod persist;
mod tap;
#[cfg(test)]
pub mod test_utils;

pub use self::device::Net;
pub use self::event_handler::*;
pub use tap::Error as TapError;

#[derive(Debug)]
pub enum Error {
    /// Open tap device failed.
    TapOpen(TapError),
    /// Setting tap interface offload flags failed.
    TapSetOffload(TapError),
    /// Setting vnet header size failed.
    TapSetVnetHdrSize(TapError),
    /// Enabling tap interface failed.
    TapEnable(TapError),
    /// EventFd error.
    EventFd(io::Error),
    /// IO error.
    IO(io::Error),
    /// The VNET header is missing from the frame.
    VnetHeaderMissing,
}

pub type Result<T> = result::Result<T, Error>;
