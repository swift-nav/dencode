use std::{
    io::Read,
    ops::{Deref, DerefMut},
};

use crate::{buffer::Buffer, fuse::Fuse, sink::IterSink, Decoder};

const INITIAL_CAPACITY: usize = 8 * 1024;

#[cfg(feature = "futures")]
pin_project_lite::pin_project! {
    /// A `Stream` of messages decoded from an `AsyncRead`.
    ///
    /// # Example
    /// ```
    /// use dencode::{LinesCodec, FramedRead};
    /// use futures::TryStreamExt;
    ///
    /// let buf = b"hello\n";
    /// let mut framed = FramedRead::new(&buf[..], LinesCodec {});
    ///
    /// # futures::executor::block_on(async move {
    /// if let Some(line) = framed.try_next().await? {
    ///     assert_eq!(line, "hello\n");
    /// }
    /// # Ok::<_, std::io::Error>(())
    /// # }).unwrap();
    /// ```
    #[derive(Debug)]
    pub struct FramedRead<Io, Codec, Buf> {
        #[pin]
        inner: FramedReadImpl<Fuse<Io, Codec>, Buf>,
    }
}
#[cfg(not(feature = "futures"))]
/// A `Stream` of messages decoded from an `AsyncRead`.
///
/// # Example
/// ```
/// use dencode::{FramedRead, LinesCodec};
/// use futures::TryStreamExt;
///
/// let buf = [104, 101, 108, 108, 111, 10];
/// let mut framed = FramedRead::new(&buf[..], LinesCodec {});
///
/// # futures::executor::block_on(async move {
/// if let Some(line) = framed.try_next().await? {
///     assert_eq!(line, "hello\n");
/// }
/// # Ok::<_, std::io::Error>(())
/// # }).unwrap();
/// ```
#[derive(Debug)]
pub struct FramedRead<Io, Codec, Buf> {
    inner: FramedReadImpl<Fuse<Io, Codec>, Buf>,
}

impl<Io, Codec, Buf> FramedRead<Io, Codec, Buf>
where
    Codec: Decoder<Buf>,
    Buf: Buffer,
{
    /// Creates a new `FramedRead` transport with the given `Decoder`.
    pub fn new(inner: Io, decoder: Codec) -> Self {
        Self {
            inner: FramedReadImpl::new(Fuse::new(inner, decoder)),
        }
    }

    /// Release the I/O and Decoder
    pub fn release(self) -> (Io, Codec) {
        let fuse = self.inner.release();
        (fuse.io, fuse.codec)
    }

    /// Consumes the `FramedRead`, returning its underlying I/O stream.
    ///
    /// Note that care should be taken to not tamper with the underlying stream
    /// of data coming in as it may corrupt the stream of frames otherwise
    /// being worked with.
    pub fn into_inner(self) -> Io {
        self.release().0
    }

    /// Returns a reference to the underlying decoder.
    ///
    /// Note that care should be taken to not tamper with the underlying decoder
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn decoder(&self) -> &Codec {
        &self.inner.codec
    }

    /// Returns a mutable reference to the underlying decoder.
    ///
    /// Note that care should be taken to not tamper with the underlying decoder
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn decoder_mut(&mut self) -> &mut Codec {
        &mut self.inner.codec
    }

    /// Returns a reference to the read buffer.
    pub fn buffer(&self) -> &Buf {
        &self.inner.buffer
    }
}

impl<Io, Codec, Buf> Deref for FramedRead<Io, Codec, Buf> {
    type Target = Io;

    fn deref(&self) -> &Io {
        &self.inner
    }
}

impl<Io, Codec, Buf> DerefMut for FramedRead<Io, Codec, Buf> {
    fn deref_mut(&mut self) -> &mut Io {
        &mut self.inner
    }
}

impl<Io, Codec, Buf> Iterator for FramedRead<Io, Codec, Buf>
where
    Io: Read,
    Codec: Decoder<Buf>,
    Buf: Buffer,
{
    type Item = Result<Codec::Item, Codec::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[cfg(feature = "futures")]
pin_project_lite::pin_project! {
    #[derive(Debug)]
    pub(crate) struct FramedReadImpl<Fuse, Buf> {
        #[pin]
        inner: Fuse,
        buffer: Buf,
    }
}
#[cfg(not(feature = "futures"))]
#[derive(Debug)]
pub(crate) struct FramedReadImpl<Fuse, Buf> {
    inner: Fuse,
    buffer: Buf,
}

impl<Fuse, Buf> FramedReadImpl<Fuse, Buf>
where
    Buf: Buffer,
{
    pub(crate) fn new(inner: Fuse) -> Self {
        Self {
            inner,
            buffer: Buf::with_capacity(INITIAL_CAPACITY),
        }
    }

    pub(crate) fn release(self) -> Fuse {
        self.inner
    }

    pub(crate) fn buffer(&self) -> &Buf {
        &self.buffer
    }
}

impl<Fuse, Buf> Deref for FramedReadImpl<Fuse, Buf> {
    type Target = Fuse;

    fn deref(&self) -> &Fuse {
        &self.inner
    }
}

impl<Fuse, Buf> DerefMut for FramedReadImpl<Fuse, Buf> {
    fn deref_mut(&mut self) -> &mut Fuse {
        &mut self.inner
    }
}

impl<Fuse, Buf> Iterator for FramedReadImpl<Fuse, Buf>
where
    Fuse: Read + Decoder<Buf>,
    Buf: Buffer,
{
    type Item = Result<Fuse::Item, Fuse::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.decode(&mut self.buffer) {
            Ok(Some(item)) => return Some(Ok(item)),
            Err(e) => return Some(Err(e)),
            Ok(None) => (),
        };
        let mut buf = [0u8; INITIAL_CAPACITY];
        loop {
            let n = match self.inner.read(&mut buf) {
                Ok(n) => n,
                Err(e) => return Some(Err(e.into())),
            };
            self.buffer.extend_from_slice(&buf[..n]);
            match self.inner.decode(&mut self.buffer) {
                Ok(Some(item)) => return Some(Ok(item)),
                Ok(None) if n == 0 => return None,
                Err(e) => return Some(Err(e)),
                _ => continue,
            };
        }
    }
}

impl<Fuse, Buf, Item> IterSink<Item> for FramedReadImpl<Fuse, Buf>
where
    Fuse: IterSink<Item>,
{
    type Error = Fuse::Error;

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
mod futures_impl {
    use std::{
        io,
        pin::Pin,
        task::{Context, Poll},
    };

    use futures_core::{ready, Stream};
    use futures_io::AsyncRead;
    use futures_sink::Sink;

    use super::*;

    impl<Io, Codec, Buf> Stream for FramedRead<Io, Codec, Buf>
    where
        Io: AsyncRead + Unpin,
        Codec: Decoder<Buf>,
        Buf: Buffer,
    {
        type Item = Result<Codec::Item, Codec::Error>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.project().inner.poll_next(cx)
        }
    }

    impl<Fuse, Buf> Stream for FramedReadImpl<Fuse, Buf>
    where
        Fuse: AsyncRead + Decoder<Buf> + Unpin,
        Buf: Buffer,
    {
        type Item = Result<Fuse::Item, Fuse::Error>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();
            if let Some(item) = this.inner.decode(this.buffer)? {
                return Poll::Ready(Some(Ok(item)));
            }
            let mut buf = [0u8; INITIAL_CAPACITY];
            loop {
                let n = ready!(Pin::new(&mut this.inner).poll_read(cx, &mut buf))?;
                this.buffer.extend_from_slice(&buf[..n]);
                let ended = n == 0;
                match this.inner.decode(this.buffer)? {
                    Some(item) => return Poll::Ready(Some(Ok(item))),
                    None if ended => {
                        if this.buffer.is_empty() {
                            return Poll::Ready(None);
                        } else {
                            match this.inner.decode_eof(this.buffer)? {
                                Some(item) => return Poll::Ready(Some(Ok(item))),
                                None if this.buffer.is_empty() => return Poll::Ready(None),
                                None => {
                                    return Poll::Ready(Some(Err(io::Error::new(
                                        io::ErrorKind::UnexpectedEof,
                                        "bytes remaining in stream",
                                    )
                                    .into())));
                                }
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

    impl<Fuse, Buf, Item> Sink<Item> for FramedReadImpl<Fuse, Buf>
    where
        Fuse: Sink<Item> + Unpin,
    {
        type Error = Fuse::Error;

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
}
