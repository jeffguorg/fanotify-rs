use std::{
    ffi::CString,
    io::{Read, Write},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    ptr::null,
};

use crate::{consts::{EventFFlags, InitFlags, MarkFlags, MaskFlags}, messages::{Event, Response}};

pub struct Fanotify {
    fd: OwnedFd,
}

impl Fanotify {
    pub fn init(init_flags: InitFlags, event_fd_flags: EventFFlags) -> std::io::Result<Self> {
        Self::try_init(init_flags, event_fd_flags)
    }
    pub fn try_init(init_flags: InitFlags, event_fd_flags: EventFFlags) -> std::io::Result<Self> {
        let fd = unsafe {
            let ret = libc::fanotify_init(init_flags.bits(), event_fd_flags.bits());
            if ret == -1 {
                return Err(std::io::Error::last_os_error());
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
    ) -> std::io::Result<()> {
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
            return Err(std::io::Error::last_os_error());
        }

        Ok(())
    }

    pub fn read_events(&mut self) -> std::io::Result<Vec<Event>> {
        const BUFFER_SIZE: usize = 4096;
        let mut buffer = [0u8; BUFFER_SIZE];
        let nread = self.read(&mut buffer)?;
        Ok(Event::extract_from(&buffer[0..nread]))
    }

    pub fn write_response(&mut self, response: Response) -> std::io::Result<usize> {
        self.write(unsafe {
            std::slice::from_raw_parts(
                (&response.inner as *const libc::fanotify_response).cast(),
                size_of::<libc::fanotify_response>(),
            )
        })
    }
}


impl AsFd for Fanotify {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for Fanotify {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.fd.as_raw_fd()
    }
}

impl Read for Fanotify {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(unsafe {
            let nread = libc::read(self.fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len());
            if nread < 0 {
                return Err(std::io::Error::last_os_error());
            }
            nread as usize
        })
    }
}

impl Write for Fanotify {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(unsafe {
            let nread = libc::write(self.fd.as_raw_fd(), buf.as_ptr().cast(), buf.len());
            if nread < 0 {
                return Err(std::io::Error::last_os_error());
            }
            nread as usize
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

