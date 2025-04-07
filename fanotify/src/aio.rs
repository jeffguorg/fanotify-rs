use std::
    os::fd::{FromRawFd, OwnedFd}
;

use tokio::io::unix::AsyncFd;

#[cfg(feature = "aio-async-read-write")]
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(not(feature = "aio-async-read-write"))]
use tokio::io::Interest;

use crate::{
    consts::{EventFFlags, InitFlags},
    fanotify::Fanotify,
    messages::{Event, Response},
};

impl Fanotify<AsyncFd<Fanotify<OwnedFd>>> {
    pub fn init(init_flags: InitFlags, event_fd_flags: EventFFlags) -> std::io::Result<Self> {
        Self::try_init(init_flags, event_fd_flags)
    }

    pub fn try_init(init_flags: InitFlags, event_fd_flags: EventFFlags) -> std::io::Result<Self> {
        // add non block flag to make it work with tokio. or else it will block after first read.
        let init_flags = init_flags | InitFlags::FAN_NONBLOCK;
        let fd = unsafe {
            let ret = libc::fanotify_init(init_flags.bits(), event_fd_flags.bits());
            if ret == -1 {
                return Err(std::io::Error::last_os_error());
            }
            OwnedFd::from_raw_fd(ret)
        };
        let fan = AsyncFd::new(Fanotify { fd })?;
        Ok(Self { fd: fan })
    }

    pub async fn read_events(&mut self) -> std::io::Result<Vec<Event>> {
        #[cfg(feature = "aio-async-read-write")]
        {
            const BUFFER_SIZE: usize = 4096;
            let mut buffer = [0u8; BUFFER_SIZE];
            let nread = self.read(&mut buffer).await?;
            Ok(Event::extract_from(&buffer[0..nread]))
        }

        #[cfg(not(feature = "aio-async-read-write"))]
        self.fd
            .async_io_mut(Interest::READABLE, |r| r.read_events())
            .await
    }

    pub async fn write_response(&mut self, response: Response) -> std::io::Result<usize> {
        #[cfg(feature = "aio-async-read-write")]
        return self
            .write(unsafe {
                std::slice::from_raw_parts(
                    (&response.inner as *const libc::fanotify_response).cast(),
                    size_of::<libc::fanotify_response>(),
                )
            })
            .await;

        #[cfg(not(feature = "aio-async-read-write"))]
        self.fd
            .async_io_mut(Interest::WRITABLE, |r| r.write_response(response))
            .await
    }
}

#[cfg(feature = "aio-async-read-write")]
impl AsyncRead for Fanotify<AsyncFd<Fanotify<OwnedFd>>> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut guard = match self.get_mut().fd.poll_read_ready_mut(cx) {
            std::task::Poll::Ready(guard_result) => guard_result,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }?;
        let nread = guard.get_inner_mut().read(buf.initialize_unfilled())?;
        guard.clear_ready();

        buf.advance(nread);
        std::task::Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "aio-async-read-write")]
impl AsyncWrite for Fanotify<AsyncFd<Fanotify<OwnedFd>>> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut guard = match self.get_mut().fd.poll_write_ready_mut(cx) {
            std::task::Poll::Ready(guard_result) => guard_result,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }?;
        let nwrite = guard.get_inner_mut().write(&buf)?;
        guard.clear_ready();
        std::task::Poll::Ready(Ok(nwrite))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        // do nothing
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        // do nothing
        std::task::Poll::Ready(Ok(()))
    }
}
