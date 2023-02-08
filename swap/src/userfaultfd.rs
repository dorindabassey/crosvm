// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Provides wrapper of userfaultfd crate for vmm-swap feature.

#![deny(missing_docs)]

use std::convert::From;
use std::fs::File;
use std::fs::OpenOptions;
use std::ops::Range;
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::FromRawFd;
use std::os::unix::prelude::OpenOptionsExt;

use anyhow::Context;
use base::errno_result;
use base::info;
use base::ioctl_io_nr;
use base::ioctl_iowr_nr;
use base::ioctl_with_mut_ref;
use base::ioctl_with_val;
use base::AsRawDescriptor;
use base::AsRawDescriptors;
use base::FromRawDescriptor;
use base::RawDescriptor;
use thiserror::Error as ThisError;
use userfaultfd::Error as UffdError;
pub use userfaultfd::Event as UffdEvent;
use userfaultfd::FeatureFlags;
use userfaultfd::IoctlFlags;
use userfaultfd::Uffd;
use userfaultfd::UffdBuilder;

const DEV_USERFAULTFD_PATH: &str = "/dev/userfaultfd";
const USERFAULTFD_IOC: u32 = 0xAA;
ioctl_io_nr!(USERFAULTFD_IOC_NEW, USERFAULTFD_IOC, 0x00);
ioctl_iowr_nr!(
    UFFDIO_API,
    userfaultfd_sys::UFFDIO,
    userfaultfd_sys::_UFFDIO_API,
    userfaultfd_sys::uffdio_api
);

/// Result for Userfaultfd
pub type Result<T> = std::result::Result<T, Error>;

/// Errors for Userfaultfd
#[derive(ThisError, Debug)]
pub enum Error {
    #[error("userfaultfd error: {0:?}")]
    /// unrecoverable userfaultfd error.
    Userfaultfd(UffdError),
    #[error("copy partially succeeded: {0:?} bytes copied")]
    /// UFFDIO_COPY partillay succeed.
    PartiallyCopied(usize),
    #[error("the page is already filled")]
    /// The page is already filled.
    PageExist,
    #[error("the uffd in the corresponding process is already closed")]
    /// The corresponding process is already dead or has run exec(2).
    UffdClosed,
}

impl From<UffdError> for Error {
    fn from(e: UffdError) -> Self {
        match e {
            UffdError::PartiallyCopied(copied) => Self::PartiallyCopied(copied),
            UffdError::ZeropageFailed(errno) if errno as i32 == libc::EEXIST => Self::PageExist,
            other => Self::Userfaultfd(other),
        }
    }
}

/// Register all the regions to all the userfaultfd
///
/// # Arguments
///
/// * `regions` - the list of address range of regions.
/// * `uffds` - the reference to the list of [Userfaultfd] for all the processes which may touch the
///   `address_range` to be registered.
///
/// # Safety
///
/// Each address range in `regions` must be from guest memory.
///
/// The `uffds` must cover all the processes which may touch the `address_range`. otherwise some
/// pages are zeroed by kernel on the unregistered process instead of swapping in from the swap
/// file.
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe fn register_regions(regions: &[Range<usize>], uffds: &[Userfaultfd]) -> Result<()> {
    for address_range in regions {
        for uffd in uffds {
            // Safe because the range is from the guest memory region.
            let result = unsafe {
                uffd.register(address_range.start, address_range.end - address_range.start)
            };
            match result {
                Ok(_) => {}
                // Skip the userfaultfd for dead processes.
                Err(Error::UffdClosed) => {}
                Err(e) => {
                    return Err(e);
                }
            };
        }
    }
    Ok(())
}

/// Unregister all the regions from all the userfaultfd.
///
/// `UFFDIO_UNREGISTER` unblocks any threads currently waiting on the region and remove page fault
/// events on the region from the userfaultfd event queue.
///
/// # Arguments
///
/// * `regions` - the list of address range of regions.
/// * `uffds` - the reference to the list of registered [Userfaultfd].
pub fn unregister_regions(regions: &[Range<usize>], uffds: &[Userfaultfd]) -> Result<()> {
    for address_range in regions {
        for uffd in uffds {
            let result =
                uffd.unregister(address_range.start, address_range.end - address_range.start);
            match result {
                Ok(_) => {}
                // Skip the userfaultfd for dead processes.
                Err(Error::UffdClosed) => {}
                Err(e) => {
                    return Err(e);
                }
            };
        }
    }
    Ok(())
}

/// Factory for [Userfaultfd].
///
/// If `/dev/userfaultfd` (introduced from Linux 6.1) exists, creates userfaultfd from the dev file.
/// Otherwise use `userfaultfd(2)` to create a userfaultfd.
pub struct Factory {
    dev_file: Option<File>,
}

impl Factory {
    /// Create [Factory] and try open `/dev/userfaultfd`.
    ///
    /// If it fails to open `/dev/userfaultfd`, userfaultfd creation fallback to `userfaultfd(2)`
    /// syscall.
    pub fn new() -> Self {
        let dev_file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(DEV_USERFAULTFD_PATH);
        match dev_file {
            Ok(dev_file) => Self {
                dev_file: Some(dev_file),
            },
            Err(e) => {
                info!(
                    "Failed to open /dev/userfaultfd ({:?}), will fall back to userfaultfd(2)",
                    e
                );
                Self { dev_file: None }
            }
        }
    }

    /// Creates a new [Userfaultfd] for this process.
    pub fn create(&self) -> anyhow::Result<Userfaultfd> {
        if let Some(dev_file) = &self.dev_file {
            // Safe because ioctl(2) USERFAULTFD_IOC_NEW with does not change Rust memory safety.
            let res = unsafe {
                ioctl_with_val(
                    dev_file,
                    USERFAULTFD_IOC_NEW(),
                    (libc::O_CLOEXEC | libc::O_NONBLOCK) as libc::c_ulong,
                )
            };
            let uffd = if res < 0 {
                return errno_result().context("USERFAULTFD_IOC_NEW");
            } else {
                // Safe because the uffd is not owned by anyone in this process.
                unsafe { Userfaultfd::from_raw_descriptor(res) }
            };
            let mut api = userfaultfd_sys::uffdio_api {
                api: userfaultfd_sys::UFFD_API,
                features: (FeatureFlags::MISSING_SHMEM | FeatureFlags::EVENT_REMOVE).bits(),
                ioctls: 0,
            };
            // Safe because ioctl(2) UFFDIO_API with does not change Rust memory safety.
            let res = unsafe { ioctl_with_mut_ref(&uffd, UFFDIO_API(), &mut api) };
            if res < 0 {
                errno_result().context("UFFDIO_API")
            } else {
                Ok(uffd)
            }
        } else {
            Userfaultfd::new().context("create userfaultfd")
        }
    }
}

impl AsRawDescriptors for Factory {
    fn as_raw_descriptors(&self) -> Vec<RawDescriptor> {
        if let Some(dev_file) = &self.dev_file {
            vec![dev_file.as_raw_descriptor()]
        } else {
            Vec::new()
        }
    }
}

/// Wrapper for [`userfaultfd::Uffd`] to be used in the vmm-swap feature.
///
/// # Safety
///
/// The userfaultfd operations (`UFFDIO_COPY` and `UFFDIO_ZEROPAGE`) looks unsafe since it fills a
/// memory content directly. But they actually are not unsafe operation but `UFFDIO_REGISTER` should
/// be the unsafe operation for Rust memory safety.
///
/// According to [the Rust document](https://doc.rust-lang.org/nomicon/uninitialized.html),
///
/// > All runtime-allocated memory in a Rust program begins its life as uninitialized.
///
/// The userfaultfd operations actually does not change/overwrite the existing memory contents but
/// they just setup the "uninitialized" pages. If the page was already initialized, the userfaultfd
/// operations fail and return EEXIST error (which is not documented unfortunately). So they
/// originally does not affect the Rust memory safety.
///
/// The "uninitialized" page in this context has 2 patterns:
///
/// 1. pages which is never touched or,
/// 2. pages which is never touched after MADV_REMOVE
///
/// Filling the (1) pages with any contents should not affect the Rust memory safety.
///
/// Filling the (2) pages potentially may break the memory used by Rust. But the safety should be
/// examined at `MADV_REMOVE` and `UFFDIO_REGISTER` timing.
pub struct Userfaultfd {
    uffd: Uffd,
}

impl Userfaultfd {
    /// Creates a new userfaultfd using userfaultfd(2) syscall.
    ///
    /// This is public for tests.
    pub fn new() -> Result<Self> {
        let uffd = UffdBuilder::new()
            .close_on_exec(true)
            .non_blocking(true)
            .user_mode_only(false)
            .require_features(FeatureFlags::MISSING_SHMEM | FeatureFlags::EVENT_REMOVE)
            .create()?;
        Ok(Self { uffd })
    }

    /// Register a range of memory to the userfaultfd.
    ///
    /// After this registration, any page faults on the range will be caught by the userfaultfd.
    ///
    /// # Arguments
    ///
    /// * `addr` - the starting address of the range of memory.
    /// * `len` - the length in bytes of the range of memory.
    ///
    /// # Safety
    ///
    /// [addr, addr+len) must lie within a [MemoryMapping](base::MemoryMapping), and that mapping
    /// must live for the lifespan of the userfaultfd kernel object (which may be distinct from the
    /// `Userfaultfd` rust object in this process).
    pub unsafe fn register(&self, addr: usize, len: usize) -> Result<IoctlFlags> {
        match self.uffd.register(addr as *mut libc::c_void, len) {
            Ok(flags) => Ok(flags),
            Err(UffdError::SystemError(errno)) if errno as i32 == libc::ENOMEM => {
                // Userfaultfd returns `ENOMEM` if the corresponding process dies or run as another
                // program by `exec` system call.
                // TODO(b/267124393): Verify UFFDIO_ZEROPAGE + ESRCH as well since ENOMEM may be for
                // other reasons.
                Err(Error::UffdClosed)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Unregister a range of memory from the userfaultfd.
    ///
    /// # Arguments
    ///
    /// * `addr` - the starting address of the range of memory.
    /// * `len` - the length in bytes of the range of memory.
    pub fn unregister(&self, addr: usize, len: usize) -> Result<()> {
        match self.uffd.unregister(addr as *mut libc::c_void, len) {
            Ok(_) => Ok(()),
            Err(UffdError::SystemError(errno)) if errno as i32 == libc::ENOMEM => {
                // Userfaultfd returns `ENOMEM` if the corresponding process dies or run as another
                // program by `exec` system call.
                // TODO(b/267124393): Verify UFFDIO_ZEROPAGE + ESRCH as well since ENOMEM may be for
                // other reasons.
                Err(Error::UffdClosed)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Initialize page(s) and fill it with zero.
    ///
    /// # Arguments
    ///
    /// * `addr` - the starting address of the page(s) to be initialzed with zero.
    /// * `len` - the length in bytes of the page(s).
    /// * `wake` - whether or not to unblock the faulting thread.
    pub fn zero(&self, addr: usize, len: usize, wake: bool) -> Result<usize> {
        // safe because zeroing untouched pages does not break the Rust memory safety since "All
        // runtime-allocated memory in a Rust program begins its life as uninitialized."
        // https://doc.rust-lang.org/nomicon/uninitialized.html
        Ok(unsafe { self.uffd.zeropage(addr as *mut libc::c_void, len, wake) }?)
    }

    /// Copy the `data` to the page(s) starting from `addr`.
    ///
    /// # Arguments
    ///
    /// * `addr` - the starting address of the page(s) to be initialzed with data.
    /// * `len` - the length in bytes of the page(s).
    /// * `data` - the starting address of the content.
    /// * `wake` - whether or not to unblock the faulting thread.
    pub fn copy(&self, addr: usize, len: usize, data: *const u8, wake: bool) -> Result<usize> {
        // safe because filling untouched pages with data does not break the Rust memory safety
        // since "All runtime-allocated memory in a Rust program begins its life as uninitialized."
        // https://doc.rust-lang.org/nomicon/uninitialized.html
        Ok(unsafe {
            self.uffd.copy(
                data as *const libc::c_void,
                addr as *mut libc::c_void,
                len,
                wake,
            )
        }?)
    }

    /// Wake the faulting thread blocked by the page(s).
    ///
    /// If the page is not initialized, the thread causes a page fault again.
    ///
    /// # Arguments
    ///
    /// * `addr` - the starting address of the page(s).
    /// * `len` - the length in bytes of the page(s).
    pub fn wake(&self, addr: usize, len: usize) -> Result<()> {
        Ok(self.uffd.wake(addr as *mut libc::c_void, len)?)
    }

    /// Read an event from the userfaultfd.
    ///
    /// Return `None` immediately if no events is ready to read.
    pub fn read_event(&self) -> Result<Option<UffdEvent>> {
        Ok(self.uffd.read_event()?)
    }
}

impl From<Uffd> for Userfaultfd {
    fn from(uffd: Uffd) -> Self {
        Self { uffd }
    }
}

impl FromRawDescriptor for Userfaultfd {
    unsafe fn from_raw_descriptor(descriptor: RawDescriptor) -> Self {
        Self::from(Uffd::from_raw_fd(descriptor))
    }
}

impl AsRawDescriptor for Userfaultfd {
    fn as_raw_descriptor(&self) -> RawDescriptor {
        self.uffd.as_raw_fd()
    }
}
