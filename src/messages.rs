use std::{mem::MaybeUninit, os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd}};

use crate::consts::MaskFlags;


pub struct Event {
    pub fanotify_event_metadata: libc::fanotify_event_metadata,
    pub event_info: Vec<EventInfo>,
}

impl Event {
    fn new(
        fanotify_event_metadata: libc::fanotify_event_metadata,
        event_info: Vec<EventInfo>,
    ) -> Self {
        Self {
            fanotify_event_metadata,
            event_info,
        }
    }

    pub fn extract_from(buf: &[u8]) -> Vec<Self> {
        const EVENT_SIZE: usize = size_of::<libc::fanotify_event_metadata>();
        let nread = buf.len() as usize;

        let mut result = Vec::new();
        let mut offset = 0;
        unsafe {
            // #define FAN_EVENT_OK(meta, len) \
            //   ((long)(len) => (long)FAN_EVENT_METADATA_LEN)                && // rest buffer can contain a metadata struct
            //   (long)(meta) ->event_len >= (long)FAN_FAN_EVENT_METADATA_LEN && // struct contains valid size (not implemented)
            //   (long)(meta) ->event_len <= (long)(len)                         // struct does not read over buffer boundary (not implemented)
            while offset + EVENT_SIZE <= nread {
                let mut uninited: MaybeUninit<libc::fanotify_event_metadata> =
                    MaybeUninit::uninit();
                std::ptr::copy(
                    buf.as_ptr().add(offset),
                    uninited.as_mut_ptr().cast(),
                    EVENT_SIZE,
                );
                let event = uninited.assume_init();

                #[repr(C)]
                union fanotify_event_info {
                    header: libc::fanotify_event_info_header,
                    fid: libc::fanotify_event_info_fid,
                    pidfd: libc::fanotify_event_info_pidfd,
                    error: libc::fanotify_event_info_error,
                }

                let mut event_info = Vec::new();

                const HEADER_SIZE: usize = std::mem::size_of::<libc::fanotify_event_info_header>();

                let mut header_offset = EVENT_SIZE;
                while header_offset + HEADER_SIZE < event.event_len as usize {
                    let mut uninited_event_info: MaybeUninit<fanotify_event_info> =
                        MaybeUninit::uninit();
                    // 1. copy header
                    std::ptr::copy(
                        buf.as_ptr().add(offset + EVENT_SIZE),
                        uninited_event_info.as_mut_ptr().cast(),
                        EVENT_SIZE,
                    );
                    // 2. copy inner data
                    std::ptr::copy(
                        buf.as_ptr().add(offset + header_offset),
                        uninited_event_info.as_mut_ptr().add(header_offset).cast(),
                        (*uninited_event_info.as_ptr()).header.len as usize - HEADER_SIZE,
                    );
                    let event_info_len = (*uninited_event_info.as_ptr()).header.len as usize;
                    let event_info_type = (*uninited_event_info.as_ptr()).header.info_type;
                    // 3. save inner data according to info_type value
                    match event_info_type {
                        libc::FAN_EVENT_INFO_TYPE_FID
                        | libc::FAN_EVENT_INFO_TYPE_DFID_NAME
                        | libc::FAN_EVENT_INFO_TYPE_DFID => {
                            event_info.push(EventInfo::Fid(uninited_event_info.assume_init().fid));
                        }
                        libc::FAN_EVENT_INFO_TYPE_PIDFD => {
                            event_info
                                .push(EventInfo::PidFd(uninited_event_info.assume_init().pidfd));
                        }
                        libc::FAN_EVENT_INFO_TYPE_ERROR => {
                            event_info
                                .push(EventInfo::PidFd(uninited_event_info.assume_init().pidfd));
                        }
                        _ => {
                            panic!("unknown fan_event_info_header.type={event_info_type}")
                        }
                    }
                    header_offset += event_info_len;
                }

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
                result.push(Event::new(event, event_info));
            }
        }

        result
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

#[cfg_attr(feature="libc-extra-traits", derive(Debug))]
pub enum EventInfo {
    Fid(libc::fanotify_event_info_fid),
    PidFd(libc::fanotify_event_info_pidfd),
    Error(libc::fanotify_event_info_error),
}

// if tokio is enabled, Response need to be sendable across threads
#[cfg_attr(feature="aio", derive(Clone, Copy))]
pub struct Response {
    pub inner: libc::fanotify_response,
}

#[cfg(feature="aio")]
unsafe impl Send for Response{}

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