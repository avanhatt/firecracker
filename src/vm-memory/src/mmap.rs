// Copyright (C) 2019 Alibaba Cloud Computing. All rights reserved.
//
// Portions Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// Portions Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE-BSD-3-Clause file.
//
// SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause

//! The default implementation for the [`GuestMemory`](trait.GuestMemory.html) trait.
//!
//! This implementation is mmap-ing the memory of the guest into the current process.

use std::borrow::Borrow;
use std::io::{Read, Write};
use std::ops::Deref;
use std::result;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use vm_memory_upstream::address::Address;
use vm_memory_upstream::guest_memory::{
    self, FileOffset, GuestAddress, GuestMemory, GuestMemoryRegion, GuestUsize, MemoryRegionAddress,
};
use vm_memory_upstream::volatile_memory::{
    Error as VolatileMemoryError, VolatileMemory, VolatileSlice,
};
use vm_memory_upstream::{AtomicAccess, ByteValued, Bytes};

use vmm_sys_util::errno;

use crate::bitmap::Bitmap;

pub use vm_memory_upstream::mmap::{MmapRegion, MmapRegionError};

// Here are some things that originate in this module, and we can continue to use the upstream
// definitions/implementations.
pub use vm_memory_upstream::mmap::{check_file_offset, Error};

// The maximum number of bytes that can be read/written at a time.
static MAX_ACCESS_CHUNK: usize = 4096;

/// [`GuestMemoryRegion`](trait.GuestMemoryRegion.html) implementation that mmaps the guest's
/// memory region in the current process.
///
/// Represents a continuous region of the guest's physical memory that is backed by a mapping
/// in the virtual address space of the calling process.
#[derive(Debug)]
pub struct GuestRegionMmap {
    mapping: MmapRegion,
    guest_base: GuestAddress,
    // handles dirty page tracking
    dirty_bitmap: Option<Bitmap>,
}

impl GuestRegionMmap {
    /// Create a new memory-mapped memory region for the guest's physical memory.
    pub fn new(mapping: MmapRegion, guest_base: GuestAddress) -> result::Result<Self, Error> {
        if guest_base.0.checked_add(mapping.len() as u64).is_none() {
            return Err(Error::InvalidGuestRegion);
        }
        Ok(GuestRegionMmap {
            mapping,
            guest_base,
            dirty_bitmap: None,
        })
    }

    /// Provide the region with a dedicated bitmap to handle dirty page tracking.
    pub fn enable_dirty_page_tracking(&mut self) {
        let page_size = match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
            -1 => panic!(
                "Failed to enable dirty page tracking: {}",
                errno::Error::last()
            ),
            ps => ps as usize,
        };
        if self.dirty_bitmap.is_none() {
            self.dirty_bitmap = Some(Bitmap::new(self.len() as usize, page_size));
        }
    }

    /// Get the dirty page bitmap representative for this memory region (if any).
    pub fn dirty_bitmap(&self) -> Option<&Bitmap> {
        self.dirty_bitmap.as_ref()
    }

    /// Mark pages dirty starting from 'start_addr' and continuing for 'len' bytes.
    pub fn mark_dirty_pages(&self, start_addr: usize, len: usize) {
        if let Some(bitmap) = self.dirty_bitmap() {
            bitmap.set_addr_range(start_addr, len);
        }
    }

    // This is exclusively used for the local `Bytes` implementation.
    fn local_volatile_slice(&self) -> VolatileSlice {
        // It's safe to unwrap because we're starting at offset 0 and specify the exact
        // length of the mapping.
        self.mapping.get_slice(0, self.mapping.len()).unwrap()
    }
}

impl Deref for GuestRegionMmap {
    type Target = MmapRegion;

    fn deref(&self) -> &MmapRegion {
        &self.mapping
    }
}

fn __nondet<T>() -> T {
    unimplemented!()
}

impl Bytes<MemoryRegionAddress> for GuestRegionMmap {
    type E = guest_memory::Error;

    fn write(&self, buf: &[u8], addr: MemoryRegionAddress) -> guest_memory::Result<usize> {
        let maddr = addr.raw_value() as usize;
        let bytes = self
            .local_volatile_slice()
            .write(buf, maddr)
            .map_err(Into::<guest_memory::Error>::into)?;
        self.mark_dirty_pages(maddr, bytes);
        Ok(bytes)
    }

    fn read(&self, buf: &mut [u8], addr: MemoryRegionAddress) -> guest_memory::Result<usize> {
        let maddr = addr.raw_value() as usize;
        self.local_volatile_slice()
            .read(buf, maddr)
            .map_err(Into::into)
    }

    fn write_slice(&self, buf: &[u8], addr: MemoryRegionAddress) -> guest_memory::Result<()> {
        let maddr = addr.raw_value() as usize;
        match self.local_volatile_slice().write_slice(buf, maddr) {
            Ok(()) => {
                self.mark_dirty_pages(maddr, buf.len());
                Ok(())
            }
            Err(e) => {
                if let VolatileMemoryError::PartialBuffer { completed, .. } = e {
                    self.mark_dirty_pages(maddr, completed);
                }
                Err(e.into())
            }
        }
    }

    fn read_slice(&self, buf: &mut [u8], addr: MemoryRegionAddress) -> guest_memory::Result<()> {
        let maddr = addr.raw_value() as usize;
        self.local_volatile_slice()
            .read_slice(buf, maddr)
            .map_err(Into::into)
    }

    // Add explicit implementations for the `*_obj` methods, just in case something changes
    // with the default logic provided in `Bytes`.
    fn write_obj<T: ByteValued>(
        &self,
        val: T,
        addr: MemoryRegionAddress,
    ) -> guest_memory::Result<()> {
        // Write dispatched to write_slice.
        self.write_slice(val.as_slice(), addr)
    }

    fn read_obj<T: ByteValued>(&self, addr: MemoryRegionAddress) -> guest_memory::Result<T> {
        // __nondet()
        let mut result: T = Default::default();
        // Read dispatched to `read_slice`.
        self.read_slice(result.as_mut_slice(), addr).map(|_| result)
    }

    fn read_from<F>(
        &self,
        addr: MemoryRegionAddress,
        src: &mut F,
        count: usize,
    ) -> guest_memory::Result<usize>
    where
        F: Read,
    {
        let maddr = addr.raw_value() as usize;
        let bytes = self
            .local_volatile_slice()
            .read_from::<F>(maddr, src, count)
            .map_err(Into::<guest_memory::Error>::into)?;
        self.mark_dirty_pages(maddr, bytes);
        Ok(bytes)
    }

    fn read_exact_from<F>(
        &self,
        addr: MemoryRegionAddress,
        src: &mut F,
        count: usize,
    ) -> guest_memory::Result<()>
    where
        F: Read,
    {
        let maddr = addr.raw_value() as usize;
        self.local_volatile_slice()
            .read_exact_from::<F>(maddr, src, count)
            .map_err(Into::<guest_memory::Error>::into)?;
        self.mark_dirty_pages(maddr, count);
        Ok(())
    }

    fn write_to<F>(
        &self,
        addr: MemoryRegionAddress,
        dst: &mut F,
        count: usize,
    ) -> guest_memory::Result<usize>
    where
        F: Write,
    {
        let maddr = addr.raw_value() as usize;
        self.local_volatile_slice()
            .write_to::<F>(maddr, dst, count)
            .map_err(Into::into)
    }

    fn write_all_to<F>(
        &self,
        addr: MemoryRegionAddress,
        dst: &mut F,
        count: usize,
    ) -> guest_memory::Result<()>
    where
        F: Write,
    {
        let maddr = addr.raw_value() as usize;
        self.local_volatile_slice()
            .write_all_to::<F>(maddr, dst, count)
            .map_err(Into::into)
    }

    fn store<T: AtomicAccess>(
        &self,
        _val: T,
        _addr: MemoryRegionAddress,
        _order: Ordering,
    ) -> guest_memory::Result<()> {
        // We do not use this.
        Err(guest_memory::Error::HostAddressNotAvailable)
    }

    fn load<T: AtomicAccess>(
        &self,
        _addr: MemoryRegionAddress,
        _order: Ordering,
    ) -> guest_memory::Result<T> {
        // We do not use this.
        Err(guest_memory::Error::HostAddressNotAvailable)
    }
}

impl GuestMemoryRegion for GuestRegionMmap {
    fn len(&self) -> GuestUsize {
        self.mapping.len() as GuestUsize
    }

    fn start_addr(&self) -> GuestAddress {
        self.guest_base
    }

    fn file_offset(&self) -> Option<&FileOffset> {
        self.mapping.file_offset()
    }

    // TODO: This implementation is temporary.
    // We need to return None here once we refactor vsock.
    unsafe fn as_slice(&self) -> Option<&[u8]> {
        // This is safe because we mapped the area at addr ourselves, so this slice will not
        // overflow. However, it is possible to alias.
        Some(std::slice::from_raw_parts(
            self.mapping.as_ptr(),
            self.mapping.size(),
        ))
    }

    // TODO: This implementation is temporary.
    // We need to return None here once we refactor vsock.
    #[allow(clippy::mut_from_ref)]
    unsafe fn as_mut_slice(&self) -> Option<&mut [u8]> {
        // This is safe because we mapped the area at addr ourselves, so this slice will not
        // overflow. However, it is possible to alias.
        Some(std::slice::from_raw_parts_mut(
            self.mapping.as_ptr(),
            self.mapping.size(),
        ))
    }

    fn get_host_address(&self, addr: MemoryRegionAddress) -> guest_memory::Result<*mut u8> {
        // Not sure why wrapping_offset is not unsafe.  Anyway this
        // is safe because we've just range-checked addr using check_address.
        self.check_address(addr)
            .ok_or(guest_memory::Error::InvalidBackendAddress)
            .map(|addr| self.as_ptr().wrapping_offset(addr.raw_value() as isize))
    }

    // TODO: This implementation is temporary.
    // We need to return None here once we refactor vsock.
    fn get_slice(
        &self,
        offset: MemoryRegionAddress,
        count: usize,
    ) -> guest_memory::Result<VolatileSlice> {
        let slice = self.mapping.get_slice(offset.raw_value() as usize, count)?;
        Ok(slice)
    }

    fn as_volatile_slice(&self) -> guest_memory::Result<VolatileSlice> {
        // We do not use this.
        Err(guest_memory::Error::HostAddressNotAvailable)
    }
}

/// [`GuestMemory`](trait.GuestMemory.html) implementation that mmaps the guest's memory
/// in the current process.
///
/// Represents the entire physical memory of the guest by tracking all its memory regions.
/// Each region is an instance of `GuestRegionMmap`, being backed by a mapping in the
/// virtual address space of the calling process.
#[derive(Clone, Debug, Default)]
pub struct GuestMemoryMmap {
    regions: Vec<Arc<GuestRegionMmap>>,
}

impl GuestMemoryMmap {
    /// Creates an empty `GuestMemoryMmap` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a container and allocates anonymous memory for guest memory regions.
    ///
    /// Valid memory regions are specified as a slice of (Address, Size) tuples sorted by Address.
    pub fn from_ranges(ranges: &[(GuestAddress, usize)]) -> result::Result<Self, Error> {
        Self::from_ranges_with_files(ranges.iter().map(|r| (r.0, r.1, None)), false)
    }

    /// Creates a container, allocates anonymous memory for guest memory regions and enables dirty
    /// page tracking.
    ///
    /// Valid memory regions are specified as a slice of (Address, Size) tuples sorted by Address.
    pub fn from_ranges_with_tracking(
        ranges: &[(GuestAddress, usize)],
    ) -> result::Result<Self, Error> {
        Self::from_ranges_with_files(ranges.iter().map(|r| (r.0, r.1, None)), true)
    }

    /// Creates a container and allocates anonymous memory for guest memory regions.
    ///
    /// # Arguments
    ///
    /// * 'ranges' - Iterator over a sequence of (Address, Size, Option<FileOffset>)
    ///              tuples sorted by Address.
    /// * 'track_dirty_pages' - Whether or not dirty page tracking is enabled.
    ///                         If set, it creates a dedicated bitmap for tracing memory writes
    ///                         specific to every region.
    pub fn from_ranges_with_files<A, T>(
        ranges: T,
        track_dirty_pages: bool,
    ) -> result::Result<Self, Error>
    where
        A: Borrow<(GuestAddress, usize, Option<FileOffset>)>,
        T: IntoIterator<Item = A>,
    {
        Self::from_regions(
            ranges
                .into_iter()
                .map(|x| {
                    let guest_base = x.borrow().0;
                    let size = x.borrow().1;

                    if let Some(ref f_off) = x.borrow().2 {
                        MmapRegion::from_file(f_off.clone(), size)
                    } else {
                        MmapRegion::new(size)
                    }
                    .map_err(Error::MmapRegion)
                    .and_then(|r| {
                        let mut mmap = GuestRegionMmap::new(r, guest_base)?;
                        if track_dirty_pages {
                            mmap.enable_dirty_page_tracking();
                        }
                        Ok(mmap)
                    })
                })
                .collect::<result::Result<Vec<_>, Error>>()?,
        )
    }

    /// Creates a new `GuestMemoryMmap` from a vector of regions.
    ///
    /// # Arguments
    ///
    /// * `regions` - The vector of regions.
    ///               The regions shouldn't overlap and they should be sorted
    ///               by the starting address.
    pub fn from_regions(mut regions: Vec<GuestRegionMmap>) -> result::Result<Self, Error> {
        Self::from_arc_regions(regions.drain(..).map(Arc::new).collect())
    }

    /// Creates a new `GuestMemoryMmap` from a vector of Arc regions.
    ///
    /// Similar to the constructor from_regions() as it returns a
    /// GuestMemoryMmap. The need for this constructor is to provide a way for
    /// consumer of this API to create a new GuestMemoryMmap based on existing
    /// regions coming from an existing GuestMemoryMmap instance.
    ///
    /// # Arguments
    ///
    /// * `regions` - The vector of Arc regions.
    ///               The regions shouldn't overlap and they should be sorted
    ///               by the starting address.
    pub fn from_arc_regions(regions: Vec<Arc<GuestRegionMmap>>) -> result::Result<Self, Error> {
        if regions.is_empty() {
            return Err(Error::NoMemoryRegion);
        }

        for window in regions.windows(2) {
            let prev = &window[0];
            let next = &window[1];

            if prev.start_addr() > next.start_addr() {
                return Err(Error::UnsortedMemoryRegions);
            }

            if prev.last_addr() >= next.start_addr() {
                return Err(Error::MemoryRegionOverlap);
            }
        }

        Ok(Self { regions })
    }

    /// Insert a region into the `GuestMemoryMmap` object and return a new `GuestMemoryMmap`.
    ///
    /// # Arguments
    /// * `region` - the memory region to insert into the guest memory object.
    pub fn insert_region(
        &self,
        mut region: GuestRegionMmap,
    ) -> result::Result<GuestMemoryMmap, Error> {
        let dirty_page_tracking = self.is_dirty_tracking_enabled();
        if dirty_page_tracking {
            region.enable_dirty_page_tracking();
        } else {
            region.dirty_bitmap = None;
        }

        let mut regions = self.regions.clone();
        regions.push(Arc::new(region));
        regions.sort_by_key(|x| x.start_addr());

        Self::from_arc_regions(regions)
    }

    /// Remove a region into the `GuestMemoryMmap` object and return a new `GuestMemoryMmap`
    /// on success, together with the removed region.
    ///
    /// # Arguments
    /// * `base` - base address of the region to be removed
    /// * `size` - size of the region to be removed
    pub fn remove_region(
        &self,
        base: GuestAddress,
        size: GuestUsize,
    ) -> result::Result<(GuestMemoryMmap, Arc<GuestRegionMmap>), Error> {
        if let Ok(region_index) = self.regions.binary_search_by_key(&base, |x| x.start_addr()) {
            if self.regions.get(region_index).unwrap().size() as GuestUsize == size {
                let mut regions = self.regions.clone();
                let region = regions.remove(region_index);
                return Ok((Self { regions }, region));
            }
        }

        Err(Error::InvalidGuestRegion)
    }

    /// Return true if dirty page tracking is enabled for `GuestMemoryMmap`, and else otherwise.
    pub fn is_dirty_tracking_enabled(&self) -> bool {
        self.regions.iter().all(|r| r.dirty_bitmap().is_some())
    }

    pub fn read_from<F>(
        &self,
        addr: GuestAddress,
        src: &mut F,
        count: usize,
    ) -> result::Result<usize, vm_memory_upstream::guest_memory::Error>
    where
        F: Read,
    {
        self.try_access(
            count,
            addr,
            |_offset,
             len,
             caddr,
             region|
             -> result::Result<usize, vm_memory_upstream::guest_memory::Error> {
                let len = std::cmp::min(len, MAX_ACCESS_CHUNK);
                let mut buf = vec![0u8; len].into_boxed_slice();
                loop {
                    match src.read(&mut buf[..]) {
                        Ok(bytes_read) => {
                            let bytes_written = region.write(&buf[0..bytes_read], caddr)?;
                            assert_eq!(bytes_written, bytes_read);
                            break Ok(bytes_read);
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(e) => break Err(vm_memory_upstream::guest_memory::Error::IOError(e)),
                    }
                }
            },
        )
    }

    pub fn read_exact_from<F>(
        &self,
        addr: GuestAddress,
        src: &mut F,
        count: usize,
    ) -> result::Result<(), vm_memory_upstream::guest_memory::Error>
    where
        F: Read,
    {
        let res = self.read_from(addr, src, count)?;
        if res != count {
            return Err(vm_memory_upstream::guest_memory::Error::PartialBuffer {
                expected: count,
                completed: res,
            });
        }
        Ok(())
    }
}

impl GuestMemory for GuestMemoryMmap {
    type R = GuestRegionMmap;

    fn num_regions(&self) -> usize {
        self.regions.len()
    }

    fn find_region(&self, addr: GuestAddress) -> Option<&GuestRegionMmap> {
        let index = match self.regions.binary_search_by_key(&addr, |x| x.start_addr()) {
            Ok(x) => Some(x),
            // Within the closest region with starting address < addr
            Err(x) if (x > 0 && addr <= self.regions[x - 1].last_addr()) => Some(x - 1),
            _ => None,
        };
        index.map(|x| self.regions[x].as_ref())
    }

    fn with_regions<F, E>(&self, cb: F) -> result::Result<(), E>
    where
        F: Fn(usize, &Self::R) -> result::Result<(), E>,
    {
        for (index, region) in self.regions.iter().enumerate() {
            cb(index, region)?;
        }
        Ok(())
    }

    fn with_regions_mut<F, E>(&self, mut cb: F) -> result::Result<(), E>
    where
        F: FnMut(usize, &Self::R) -> result::Result<(), E>,
    {
        for (index, region) in self.regions.iter().enumerate() {
            cb(index, region)?;
        }
        Ok(())
    }

    fn map_and_fold<F, G, T>(&self, init: T, mapf: F, foldf: G) -> T
    where
        F: Fn((usize, &Self::R)) -> T,
        G: Fn(T, T) -> T,
    {
        self.regions
            .iter()
            .enumerate()
            .map(|(idx, region)| mapf((idx, region.as_ref())))
            .fold(init, foldf)
    }
}
