use std::{
    ffi::CString,
    mem::MaybeUninit,
    os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    ptr::null,
};

use crate::{
    consts::{EventFFlags, InitFlags, MarkFlags, MaskFlags},
    error::Errno,
};

pub struct Fanotify {
    fd: OwnedFd,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("cannot convert path into c string: {0}")]
    PathError(#[from] std::ffi::NulError),

    #[error("fanotify returned errno: {0}")]
    FanotifyError(#[from] Errno),
}

impl Fanotify {
    pub fn init(init_flags: InitFlags, event_fd_flags: EventFFlags) -> Result<Self, Error> {
        Self::try_init(init_flags, event_fd_flags)
    }
    pub fn try_init(init_flags: InitFlags, event_fd_flags: EventFFlags) -> Result<Self, Error> {
        let fd = unsafe {
            let ret = libc::fanotify_init(init_flags.bits(), event_fd_flags.bits());
            if ret == -1 {
                return Err(Error::FanotifyError(Errno::errno()));
            }
            OwnedFd::from_raw_fd(ret)
        };
        Ok(Self { fd })
    }

    pub fn mark<P: Into<String>>(
        &self,
        operation: MarkFlags,
        mask: MaskFlags,
        dirfd: Option<BorrowedFd>,
        path: Option<P>,
    ) -> Result<(), Error> {
        let dirfd = match dirfd {
            Some(fd) => fd.as_raw_fd(),
            None => libc::AT_FDCWD,
        };
        let result = unsafe {
            // hold it here to prevent drop. don't merge two matches
            if let Some(path) = path {
                let path: String = path.into();
                let cstr = CString::new(path)?;
                libc::fanotify_mark(
                    self.fd.as_raw_fd(),
                    operation.bits(),
                    mask.bits(),
                    dirfd,
                    cstr.as_ptr(),
                )
            } else {
                libc::fanotify_mark(
                    self.fd.as_raw_fd(),
                    operation.bits(),
                    mask.bits(),
                    dirfd,
                    null(),
                )
            }
        };

        if result != 0 {
            return Err(Error::FanotifyError(Errno::errno()));
        }

        Ok(())
    }

    pub fn read_events(&self) -> Result<Vec<Event>, Error> {
        const BUFFER_SIZE: usize = 4096;
        const EVENT_SIZE: usize = size_of::<libc::fanotify_event_metadata>();
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut result = Vec::new();
        unsafe {
            let nread = libc::read(self.fd.as_raw_fd(), buffer.as_mut_ptr().cast(), BUFFER_SIZE);
            if nread < 0 {
                return Err(Error::FanotifyError(Errno::new(nread as i32)));
            }
            let nread = nread as usize;
            let mut offset = 0;
            // #define FAN_EVENT_OK(meta, len) \
            //   ((long)(len) => (long)FAN_EVENT_METADATA_LEN)                && // rest buffer can contain a metadata struct
            //   (long)(meta) ->event_len >= (long)FAN_FAN_EVENT_METADATA_LEN && // struct contains valid size (not implemented)
            //   (long)(meta) ->event_len <= (long)(len)                         // struct does not read over buffer boundary (not implemented)
            while offset + EVENT_SIZE <= nread {
                let mut uninited: MaybeUninit<libc::fanotify_event_metadata> = MaybeUninit::uninit();
                std::ptr::copy(
                    buffer.as_ptr().add(offset),
                    uninited.as_mut_ptr().cast(),
                    EVENT_SIZE,
                );
                let event = uninited.assume_init();

                // #define FAN_EVENT_NEXT(meta, len) ((len) -= (meta)->event_len, (struct fanotify_event_metadata*)(((char*)(meta)) + (meta) -> event_len)
                // meta = FAN_EVENT_NEXT(meta, len) translate to:
                //   len -= meta->event_len; // shrink rest length
                //   ,                       // comma operator, evaluate first express, but not using its result
                // 
                //   meta = (struct fanotify_event_metadata*) (  // cast pointer back to metadata type
                //      ((char*)(meta))                          // discard metadata type to increase pointer by 1
                //      + (meta) -> event_len                    // add event_len to move to next
                //   );
                offset += event.event_len as usize;
                result.push(Event::new(event));
            }
        }

        Ok(result)
    }

    pub fn write_response(&self, response: Response) -> Result<(), Errno> {
        let n = unsafe {
            libc::write(
                self.fd.as_raw_fd(),
                (&response.inner as *const libc::fanotify_response).cast(),
                size_of::<libc::fanotify_response>(),
            )
        };
        if n == -1 {
            return Err(Errno::errno());
        }
        Ok(())
    }
}

pub struct Event {
    pub fanotify_event_metadata: libc::fanotify_event_metadata,
}

impl Event {
    fn new(fanotify_event_metadata: libc::fanotify_event_metadata) -> Self {
        Self {
            fanotify_event_metadata,
        }
    }
    // compatible to nix::sys::fanotify::FanotifyEvent
    pub fn metadata_version(&self) -> u8 {
        self.fanotify_event_metadata.vers
    }
    pub fn check_metadata_version(&self) -> bool {
        self.fanotify_event_metadata.vers == libc::FANOTIFY_METADATA_VERSION
    }
    pub fn fd(&self) -> Option<BorrowedFd> {
        if self.fanotify_event_metadata.fd == libc::FAN_NOFD {
            None
        } else {
            Some(unsafe { BorrowedFd::borrow_raw(self.fanotify_event_metadata.fd) })
        }
    }
    pub fn pid(&self) -> i32 {
        self.fanotify_event_metadata.pid
    }
    pub fn mask(&self) -> MaskFlags {
        MaskFlags::from_bits_truncate(self.fanotify_event_metadata.mask)
    }

    // sometimes we don't want to close the fd immediately, so we forget about it, store it somewhere, and drop it later
    // it is safe to just call this method without store it in variable, it will be dropped immediately due to the nature of rust
    pub fn forget_fd(&mut self) -> OwnedFd {
        let fd = self.fanotify_event_metadata.fd;
        self.fanotify_event_metadata.fd = libc::FAN_NOFD;

        unsafe { OwnedFd::from_raw_fd(fd) }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        if self.fanotify_event_metadata.fd == libc::FAN_NOFD {
            return;
        }

        let e = unsafe { libc::close(self.fanotify_event_metadata.fd) };
        if !std::thread::panicking() && e == libc::EBADF {
            panic!("Closing an invalid file descriptor!");
        };
    }
}

pub struct Response {
    inner: libc::fanotify_response,
}

impl Response {
    pub const FAN_ALLOW: u32 = libc::FAN_ALLOW;
    pub const FAN_DENY: u32 = libc::FAN_DENY;
    pub fn new(fd: BorrowedFd, response: u32) -> Self {
        Self {
            inner: libc::fanotify_response {
                fd: fd.as_raw_fd(),
                response,
            },
        }
    }
}
