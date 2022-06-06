use std::{
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

use crate::{
    framed_read::FramedReadImpl, framed_write::FramedWriteImpl, fuse::Fuse, Buffer, Decoder,
    Encoder, IterSink,
};

#[cfg(feature = "futures")]
pin_project_lite::pin_project! {
    /// A unified `Stream` and `Sink` interface to an underlying I/O object,
    /// using the `Encoder` and `Decoder` traits to encode and decode frames.
    ///
    /// # Example
    /// ```
    /// use bytes::Bytes;
    /// use dencode::{LinesCodec, Framed};
    /// use futures::{io::Cursor, SinkExt, TryStreamExt};
    ///
    /// # futures::executor::block_on(async move {
    /// let cur = Cursor::new(vec![0u8; 6]);
    /// let mut framed = Framed::new(cur, LinesCodec {});
    ///
    /// let item = "hello";
    /// framed.send(item).await?;
    ///
    /// // Release the I/O and codec
    /// let (cur, _) = framed.release();
    /// assert_eq!(cur.get_ref(), b"hello\n");
    /// # Ok::<_, std::io::Error>(())
    /// # }).unwrap();
    /// ```
    #[derive(Debug)]
    pub struct Framed<Io, Codec, Buf> {
        #[pin]
        inner: FramedReadImpl<FramedWriteImpl<Fuse<Io, Codec>, Buf>, Buf>,
    }
}
#[cfg(not(feature = "futures"))]
/// A unified `Stream` and `Sink` interface to an underlying I/O object,
/// using the `Encoder` and `Decoder` traits to encode and decode frames.
///
/// # Example
/// ```
/// use bytes::Bytes;
/// use dencode::{Framed, LinesCodec};
/// use futures::{io::Cursor, SinkExt, TryStreamExt};
///
/// # futures::executor::block_on(async move {
/// let cur = Cursor::new(vec![0u8; 6]);
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
#[derive(Debug)]
pub struct Framed<Io, Codec, Buf> {
    inner: FramedReadImpl<FramedWriteImpl<Fuse<Io, Codec>, Buf>, Buf>,
}

impl<Io, Codec, Buf> Deref for Framed<Io, Codec, Buf> {
    type Target = Io;

    fn deref(&self) -> &Io {
        &self.inner
    }
}

impl<Io, Codec, Buf> DerefMut for Framed<Io, Codec, Buf> {
    fn deref_mut(&mut self) -> &mut Io {
        &mut self.inner
    }
}

impl<Io, Codec, Buf> Framed<Io, Codec, Buf>
where
    Buf: Buffer,
{
    /// Creates a new `Framed` transport with the given codec.
    /// A codec is a type which implements `Decoder` and `Encoder`.
    pub fn new(inner: Io, codec: Codec) -> Self {
        Self {
            inner: FramedReadImpl::new(FramedWriteImpl::new(Fuse::new(inner, codec))),
        }
    }

    /// Release the I/O and Codec
    pub fn release(self) -> (Io, Codec) {
        let fuse = self.inner.release().release();
        (fuse.io, fuse.codec)
    }

    /// Consumes the `Framed`, returning its underlying I/O stream.
    ///
    /// Note that care should be taken to not tamper with the underlying stream
    /// of data coming in as it may corrupt the stream of frames otherwise
    /// being worked with.
    pub fn into_inner(self) -> Io {
        self.release().0
    }

    /// Returns a reference to the underlying codec wrapped by
    /// `Framed`.
    ///
    /// Note that care should be taken to not tamper with the underlying codec
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn codec(&self) -> &Codec {
        &self.inner.codec
    }

    /// Returns a mutable reference to the underlying codec wrapped by
    /// `Framed`.
    ///
    /// Note that care should be taken to not tamper with the underlying codec
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn codec_mut(&mut self) -> &mut Codec {
        &mut self.inner.codec
    }

    /// Returns a reference to the read buffer.
    pub fn read_buffer(&self) -> &Buf {
        self.inner.buffer()
    }
}

impl<Io, Codec, Buf> Iterator for Framed<Io, Codec, Buf>
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

impl<Io, Codec, Buf, Item> IterSink<Item> for Framed<Io, Codec, Buf>
where
    Io: Write,
    Codec: Encoder<Item, Buf>,
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
mod futures_impl {
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    use futures_core::Stream;
    use futures_io::{AsyncRead, AsyncWrite};
    use futures_sink::Sink;

    use crate::{Buffer, Decoder, Encoder, Framed};

    impl<Io, Codec, Buf> Stream for Framed<Io, Codec, Buf>
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

    impl<Io, Codec, Buf, Item> Sink<Item> for Framed<Io, Codec, Buf>
    where
        Io: AsyncWrite + Unpin,
        Codec: Encoder<Item, Buf>,
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
}
