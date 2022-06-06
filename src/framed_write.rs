use std::{
    io,
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

use crate::{fuse::Fuse, sink::IterSink, Buffer, Encoder};

// 2^17 bytes, which is slightly over 60% of the default
// TCP send buffer size (SO_SNDBUF)
const DEFAULT_SEND_HIGH_WATER_MARK: usize = 131072;

#[cfg(feature = "futures")]
pin_project_lite::pin_project! {
    /// A `Sink` of frames encoded to an `AsyncWrite`.
    ///
    /// # Example
    /// ```
    /// use dencode::{LinesCodec, FramedWrite};
    /// use futures::SinkExt;
    ///
    /// # futures::executor::block_on(async move {
    /// let mut buf = Vec::new();
    /// let mut framed = FramedWrite::new(&mut buf, LinesCodec {});
    ///
    /// let item = "hello";
    /// framed.send(item).await?;
    ///
    /// assert_eq!(&buf[..], "hello\n".as_bytes());
    /// # Ok::<_, std::io::Error>(())
    /// # }).unwrap();
    /// ```
    #[derive(Debug)]
    pub struct FramedWrite<Io, Codec, Buf> {
        #[pin]
        inner: FramedWriteImpl<Fuse<Io, Codec>, Buf>,
    }
}
#[cfg(not(feature = "futures"))]
/// A `Sink` of frames encoded to an `AsyncWrite`.
///
/// # Example
/// ```
/// use dencode::{FramedWrite, LinesCodec};
/// use futures::SinkExt;
///
/// # futures::executor::block_on(async move {
/// let mut buf = Vec::new();
/// let mut framed = FramedWrite::new(&mut buf, LinesCodec {});
///
/// let item = "hello";
/// framed.send(item).await?;
///
/// assert_eq!(&buf[..], "hello\n".as_bytes());
/// # Ok::<_, std::io::Error>(())
/// # }).unwrap();
/// ```
#[derive(Debug)]
pub struct FramedWrite<Io, Codec, Buf> {
    inner: FramedWriteImpl<Fuse<Io, Codec>, Buf>,
}

impl<Io, Codec, Buf> FramedWrite<Io, Codec, Buf>
where
    Buf: Buffer,
{
    /// Creates a new `FramedWrite` transport with the given `Encoder`.
    pub fn new(inner: Io, encoder: Codec) -> Self {
        Self {
            inner: FramedWriteImpl::new(Fuse::new(inner, encoder)),
        }
    }

    /// High-water mark for writes, in bytes
    ///
    /// The send *high-water mark* prevents the `FramedWrite`
    /// from accepting additional messages to send when its
    /// buffer exceeds this length, in bytes. Attempts to enqueue
    /// additional messages will be deferred until progress is
    /// made on the underlying `AsyncWrite`. This applies
    /// back-pressure on fast senders and prevents unbounded
    /// buffer growth.
    ///
    /// See [`set_send_high_water_mark()`](#method.set_send_high_water_mark).
    pub fn send_high_water_mark(&self) -> usize {
        self.inner.high_water_mark
    }

    /// Sets high-water mark for writes, in bytes
    ///
    /// The send *high-water mark* prevents the `FramedWrite`
    /// from accepting additional messages to send when its
    /// buffer exceeds this length, in bytes. Attempts to enqueue
    /// additional messages will be deferred until progress is
    /// made on the underlying `AsyncWrite`. This applies
    /// back-pressure on fast senders and prevents unbounded
    /// buffer growth.
    ///
    /// The default high-water mark is 2^17 bytes. Applications
    /// which desire low latency may wish to reduce this value.
    /// There is little point to increasing this value beyond
    /// your socket's `SO_SNDBUF` size. On linux, this defaults
    /// to 212992 bytes but is user-adjustable.
    pub fn set_send_high_water_mark(&mut self, hwm: usize) {
        self.inner.high_water_mark = hwm;
    }

    /// Release the I/O and Encoder
    pub fn release(self) -> (Io, Codec) {
        let fuse = self.inner.release();
        (fuse.io, fuse.codec)
    }

    /// Consumes the `FramedWrite`, returning its underlying I/O stream.
    ///
    /// Note that care should be taken to not tamper with the underlying stream
    /// of data coming in as it may corrupt the stream of frames otherwise
    /// being worked with.
    pub fn into_inner(self) -> Io {
        self.release().0
    }

    /// Returns a reference to the underlying encoder.
    ///
    /// Note that care should be taken to not tamper with the underlying encoder
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn encoder(&self) -> &Codec {
        &self.inner.codec
    }

    /// Returns a mutable reference to the underlying encoder.
    ///
    /// Note that care should be taken to not tamper with the underlying encoder
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn encoder_mut(&mut self) -> &mut Codec {
        &mut self.inner.codec
    }
}

impl<Io, Codec, Buf> Deref for FramedWrite<Io, Codec, Buf> {
    type Target = Io;

    fn deref(&self) -> &Io {
        &self.inner
    }
}

impl<Io, Codec, Buf> DerefMut for FramedWrite<Io, Codec, Buf> {
    fn deref_mut(&mut self) -> &mut Io {
        &mut self.inner
    }
}

impl<Io, Codec, Buf, Item> IterSink<Item> for FramedWrite<Io, Codec, Buf>
where
    Io: Write,
    Codec: Encoder<Buf, Item>,
    Buf: Buffer,
{
    type Error = Codec::Error;

    fn start_send(&mut self, item: Item) -> Result<(), Self::Error> {
        self.inner.start_send(item)
    }

    fn ready(&mut self) -> Result<(), Self::Error> {
        self.inner.ready()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

#[cfg(feature = "futures")]
pin_project_lite::pin_project! {
    #[derive(Debug)]
    pub(crate) struct FramedWriteImpl<Fuse, Buf> {
        #[pin]
        pub(crate) inner: Fuse,
        pub(crate) high_water_mark: usize,
        buffer: Buf,
    }
}
#[cfg(not(feature = "futures"))]
#[derive(Debug)]
pub(crate) struct FramedWriteImpl<Fuse, Buf> {
    pub(crate) inner: Fuse,
    pub(crate) high_water_mark: usize,
    buffer: Buf,
}

impl<Fuse, Buf> FramedWriteImpl<Fuse, Buf>
where
    Buf: Buffer,
{
    pub(crate) fn new(inner: Fuse) -> FramedWriteImpl<Fuse, Buf> {
        FramedWriteImpl {
            inner,
            high_water_mark: DEFAULT_SEND_HIGH_WATER_MARK,
            buffer: Buf::with_capacity(1028 * 8),
        }
    }

    pub(crate) fn release(self) -> Fuse {
        self.inner
    }
}

impl<Fuse, Buf> Deref for FramedWriteImpl<Fuse, Buf> {
    type Target = Fuse;

    fn deref(&self) -> &Fuse {
        &self.inner
    }
}

impl<Fuse, Buf> DerefMut for FramedWriteImpl<Fuse, Buf> {
    fn deref_mut(&mut self) -> &mut Fuse {
        &mut self.inner
    }
}

impl<Fuse: Read, Buf> Read for FramedWriteImpl<Fuse, Buf> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<Fuse, Buf, Item> IterSink<Item> for FramedWriteImpl<Fuse, Buf>
where
    Fuse: Write + Encoder<Buf, Item>,
    Buf: Buffer,
{
    type Error = Fuse::Error;

    fn start_send(&mut self, item: Item) -> Result<(), Self::Error> {
        self.inner.encode(item, &mut self.buffer)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        while !self.buffer.is_empty() {
            let num_write = self.inner.write(self.buffer.as_slice())?;

            if num_write == 0 {
                return Err(err_eof().into());
            }

            self.buffer.advance(num_write);
        }

        self.inner.flush().map_err(Into::into)
    }

    fn ready(&mut self) -> Result<(), Self::Error> {
        while self.buffer.len() >= self.high_water_mark {
            let num_write = self.inner.write(self.buffer.as_slice())?;

            if num_write == 0 {
                return Err(err_eof().into());
            }

            self.buffer.advance(num_write);
        }
        Ok(())
    }
}

#[cfg(feature = "futures")]
mod futures_impl {
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    use futures_core::ready;
    use futures_io::{AsyncRead, AsyncWrite};
    use futures_sink::Sink;

    use super::*;

    impl<Io, Codec, Buf, Item> Sink<Item> for FramedWrite<Io, Codec, Buf>
    where
        Io: AsyncWrite + Unpin,
        Codec: Encoder<Buf, Item>,
        Buf: Buffer,
    {
        type Error = Codec::Error;

        fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.project().inner.poll_ready(cx)
        }

        fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
            self.project().inner.start_send(item)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.project().inner.poll_flush(cx)
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.project().inner.poll_close(cx)
        }
    }

    impl<Io: AsyncRead + Unpin, Buf> AsyncRead for FramedWriteImpl<Io, Buf> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            self.project().inner.poll_read(cx, buf)
        }
    }

    impl<Fuse, Buf, Item> Sink<Item> for FramedWriteImpl<Fuse, Buf>
    where
        Fuse: AsyncWrite + Encoder<Buf, Item> + Unpin,
        Buf: Buffer,
    {
        type Error = Fuse::Error;

        fn poll_ready(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            let this = &mut *self;
            while this.buffer.len() >= this.high_water_mark {
                let num_write =
                    ready!(Pin::new(&mut this.inner).poll_write(cx, this.buffer.as_slice()))?;

                if num_write == 0 {
                    return Poll::Ready(Err(err_eof().into()));
                }

                this.buffer.advance(num_write);
            }

            Poll::Ready(Ok(()))
        }

        fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
            let mut this = self.project();
            this.inner.encode(item, this.buffer)
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            let mut this = self.project();
            while !this.buffer.is_empty() {
                let num_write =
                    ready!(Pin::new(&mut this.inner).poll_write(cx, this.buffer.as_slice()))?;
                if num_write == 0 {
                    return Poll::Ready(Err(err_eof().into()));
                }
                this.buffer.advance(num_write);
            }
            this.inner.poll_flush(cx).map_err(Into::into)
        }

        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            ready!(self.as_mut().poll_flush(cx))?;
            self.project().inner.poll_close(cx).map_err(Into::into)
        }
    }
}

fn err_eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "End of file")
}
