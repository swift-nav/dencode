use std::{
    io,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "futures")]
pin_project_lite::pin_project! {
    #[derive(Debug)]
    pub(crate) struct Fuse<T, U> {
        #[pin]
        pub(crate) io: T,
        pub(crate) codec: U,
    }
}
#[cfg(not(feature = "futures"))]
#[derive(Debug)]
pub(crate) struct Fuse<T, U> {
    pub(crate) io: T,
    pub(crate) codec: U,
}

impl<T, U> Fuse<T, U> {
    pub(crate) fn new(t: T, u: U) -> Self {
        Self { io: t, codec: u }
    }
}

impl<T, U> Deref for Fuse<T, U> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.io
    }
}

impl<T, U> DerefMut for Fuse<T, U> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.io
    }
}

impl<T, U> io::Read for Fuse<T, U>
where
    T: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read(buf)
    }
}

impl<T, U> io::Write for Fuse<T, U>
where
    T: io::Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.io.flush()
    }
}

#[cfg(feature = "futures")]
mod futures_impl {
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    use futures_io::{AsyncRead, AsyncWrite};

    use super::*;

    impl<T, U> AsyncRead for Fuse<T, U>
    where
        T: AsyncRead + Unpin,
    {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            self.project().io.poll_read(cx, buf)
        }
    }

    impl<T, U> AsyncWrite for Fuse<T, U>
    where
        T: AsyncWrite + Unpin,
    {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.project().io.poll_write(cx, buf)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.project().io.poll_flush(cx)
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.project().io.poll_close(cx)
        }
    }
}
