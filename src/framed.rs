use std::{
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

use bytes::BytesMut;

use crate::{
    framed_read::FramedReadImpl, framed_write::FramedWriteImpl, fuse::Fuse, Decoder, Encoder,
    IterSink,
};

/// A unified `Stream` and `Sink` interface to an underlying I/O object,
/// using the `Encoder` and `Decoder` traits to encode and decode frames.
///
/// # Example
/// ```
/// use bytes::Bytes;
/// use dencode::{BytesCodec, Framed};
/// use futures::{io::Cursor, SinkExt, TryStreamExt};
///
/// # futures::executor::block_on(async move {
/// let cur = Cursor::new(vec![0u8; 12]);
/// let mut framed = Framed::new(cur, BytesCodec {});
///
/// // Send bytes to `buf` through the `BytesCodec`
/// let bytes = Bytes::from("Hello world!");
/// framed.send(bytes).await?;
///
/// // Release the I/O and codec
/// let (cur, _) = framed.release();
/// assert_eq!(cur.get_ref(), b"Hello world!");
/// # Ok::<_, std::io::Error>(())
/// # }).unwrap();
/// ```
#[cfg_attr(feature = "async", pin_project::pin_project)]
#[derive(Debug)]
pub struct Framed<T, U> {
    #[cfg_attr(feature = "async", pin)]
    inner: FramedReadImpl<FramedWriteImpl<Fuse<T, U>>>,
}

impl<T, U> Deref for Framed<T, U> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T, U> DerefMut for Framed<T, U> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T, U> Framed<T, U> {
    /// Creates a new `Framed` transport with the given codec.
    /// A codec is a type which implements `Decoder` and `Encoder`.
    pub fn new(inner: T, codec: U) -> Self {
        Self {
            inner: FramedReadImpl::new(FramedWriteImpl::new(Fuse::new(inner, codec))),
        }
    }

    /// Release the I/O and Codec
    pub fn release(self) -> (T, U) {
        let fuse = self.inner.release().release();
        (fuse.io, fuse.codec)
    }

    /// Consumes the `Framed`, returning its underlying I/O stream.
    ///
    /// Note that care should be taken to not tamper with the underlying stream
    /// of data coming in as it may corrupt the stream of frames otherwise
    /// being worked with.
    pub fn into_inner(self) -> T {
        self.release().0
    }

    /// Returns a reference to the underlying codec wrapped by
    /// `Framed`.
    ///
    /// Note that care should be taken to not tamper with the underlying codec
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn codec(&self) -> &U {
        &self.inner.codec
    }

    /// Returns a mutable reference to the underlying codec wrapped by
    /// `Framed`.
    ///
    /// Note that care should be taken to not tamper with the underlying codec
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn codec_mut(&mut self) -> &mut U {
        &mut self.inner.codec
    }

    /// Returns a reference to the read buffer.
    pub fn read_buffer(&self) -> &BytesMut {
        self.inner.buffer()
    }
}

impl<T, U> Iterator for Framed<T, U>
where
    T: Read,
    U: Decoder,
{
    type Item = Result<U::Item, U::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<T, U, I> IterSink<I> for Framed<T, U>
where
    T: Write,
    U: Encoder<I>,
{
    type Error = U::Error;

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

#[cfg(feature = "async")]
mod if_async {
    use std::{
        marker::Unpin,
        pin::Pin,
        task::{Context, Poll},
    };

    use futures_sink::Sink;
    use futures_util::{
        io::{AsyncRead, AsyncWrite},
        stream::{Stream, TryStreamExt},
    };

    use crate::{Decoder, Encoder, Framed};

    impl<T, U> Stream for Framed<T, U>
    where
        T: AsyncRead + Unpin,
        U: Decoder,
    {
        type Item = Result<U::Item, U::Error>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.inner.try_poll_next_unpin(cx)
        }
    }

    impl<T, U, I> Sink<I> for Framed<T, U>
    where
        T: AsyncWrite + Unpin,
        U: Encoder<I>,
    {
        type Error = U::Error;

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
