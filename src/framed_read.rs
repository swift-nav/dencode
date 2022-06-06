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
    /// use bytes::Bytes;
    /// use dencode::{BytesCodec, FramedRead};
    /// use futures::TryStreamExt;
    ///
    /// let buf = [3u8; 3];
    /// let mut framed = FramedRead::new(&buf[..], BytesCodec {});
    ///
    /// # futures::executor::block_on(async move {
    /// if let Some(bytes) = framed.try_next().await? {
    ///     assert_eq!(bytes, Bytes::copy_from_slice(&buf[..]));
    /// }
    /// # Ok::<_, std::io::Error>(())
    /// # }).unwrap();
    /// ```
    #[derive(Debug)]
    pub struct FramedRead<T, D, B> {
        #[pin]
        inner: FramedReadImpl<Fuse<T, D>, B>,
    }
}
#[cfg(not(feature = "futures"))]
/// A `Stream` of messages decoded from an `AsyncRead`.
///
/// # Example
/// ```
/// use bytes::Bytes;
/// use dencode::{BytesCodec, FramedRead};
/// use futures::TryStreamExt;
///
/// let buf = [3u8; 3];
/// let mut framed = FramedRead::new(&buf[..], BytesCodec {});
///
/// # futures::executor::block_on(async move {
/// if let Some(bytes) = framed.try_next().await? {
///     assert_eq!(bytes, Bytes::copy_from_slice(&buf[..]));
/// }
/// # Ok::<_, std::io::Error>(())
/// # }).unwrap();
/// ```
#[derive(Debug)]
pub struct FramedRead<T, D, B> {
    inner: FramedReadImpl<Fuse<T, D>, B>,
}

impl<T, D, B> FramedRead<T, D, B>
where
    D: Decoder<B>,
    B: Buffer,
{
    /// Creates a new `FramedRead` transport with the given `Decoder`.
    pub fn new(inner: T, decoder: D) -> Self {
        Self {
            inner: FramedReadImpl::new(Fuse::new(inner, decoder)),
        }
    }

    /// Release the I/O and Decoder
    pub fn release(self) -> (T, D) {
        let fuse = self.inner.release();
        (fuse.io, fuse.codec)
    }

    /// Consumes the `FramedRead`, returning its underlying I/O stream.
    ///
    /// Note that care should be taken to not tamper with the underlying stream
    /// of data coming in as it may corrupt the stream of frames otherwise
    /// being worked with.
    pub fn into_inner(self) -> T {
        self.release().0
    }

    /// Returns a reference to the underlying decoder.
    ///
    /// Note that care should be taken to not tamper with the underlying decoder
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn decoder(&self) -> &D {
        &self.inner.codec
    }

    /// Returns a mutable reference to the underlying decoder.
    ///
    /// Note that care should be taken to not tamper with the underlying decoder
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn decoder_mut(&mut self) -> &mut D {
        &mut self.inner.codec
    }

    /// Returns a reference to the read buffer.
    pub fn buffer(&self) -> &B {
        &self.inner.buffer
    }
}

impl<T, D, B> Deref for FramedRead<T, D, B> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T, D, B> DerefMut for FramedRead<T, D, B> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T, D, B> Iterator for FramedRead<T, D, B>
where
    T: Read,
    D: Decoder<B>,
    B: Buffer,
{
    type Item = Result<D::Item, D::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[cfg(feature = "futures")]
pin_project_lite::pin_project! {
    #[derive(Debug)]
    pub(crate) struct FramedReadImpl<T, B> {
        #[pin]
        inner: T,
        buffer: B,
    }
}
#[cfg(not(feature = "futures"))]
#[derive(Debug)]
pub(crate) struct FramedReadImpl<T, B> {
    inner: T,
    buffer: B,
}

impl<T, B> FramedReadImpl<T, B>
where
    B: Buffer,
{
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner,
            buffer: B::with_capacity(INITIAL_CAPACITY),
        }
    }

    pub(crate) fn release(self) -> T {
        self.inner
    }

    pub(crate) fn buffer(&self) -> &B {
        &self.buffer
    }
}

impl<T, B> Deref for FramedReadImpl<T, B> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T, B> DerefMut for FramedReadImpl<T, B> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T, B> Iterator for FramedReadImpl<T, B>
where
    T: Read + Decoder<B>,
    B: Buffer,
{
    type Item = Result<T::Item, T::Error>;

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

impl<T, B, I> IterSink<I> for FramedReadImpl<T, B>
where
    T: IterSink<I>,
{
    type Error = T::Error;

    fn start_send(&mut self, item: I) -> Result<(), Self::Error> {
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

    impl<T, D, B> Stream for FramedRead<T, D, B>
    where
        T: AsyncRead + Unpin,
        D: Decoder<B>,
        B: Buffer,
    {
        type Item = Result<D::Item, D::Error>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.project().inner.poll_next(cx)
        }
    }

    impl<T, B> Stream for FramedReadImpl<T, B>
    where
        T: AsyncRead + Decoder<B> + Unpin,
        B: Buffer,
    {
        type Item = Result<T::Item, T::Error>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();

            if let Some(item) = this.inner.decode(&mut this.buffer)? {
                return Poll::Ready(Some(Ok(item)));
            }

            let mut buf = [0u8; INITIAL_CAPACITY];

            loop {
                let n = ready!(Pin::new(&mut this.inner).poll_read(cx, &mut buf))?;
                this.buffer.extend_from_slice(&buf[..n]);

                let ended = n == 0;

                match this.inner.decode(&mut this.buffer)? {
                    Some(item) => return Poll::Ready(Some(Ok(item))),
                    None if ended => {
                        if this.buffer.is_empty() {
                            return Poll::Ready(None);
                        } else {
                            match this.inner.decode_eof(&mut this.buffer)? {
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

    impl<T, B, I> Sink<I> for FramedReadImpl<T, B>
    where
        T: Sink<I> + Unpin,
    {
        type Error = T::Error;

        fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.project().inner.poll_ready(cx)
        }
        fn start_send(self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
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
