use std::io;

use super::{framed_write::FramedWriteImpl, fuse::Fuse};
use crate::Buffer;

/// Decoding of frames via buffers, for use with `FramedRead`.
pub trait Decoder<Buf>
where
    Buf: Buffer,
{
    /// The type of items returned by `decode`
    type Item;
    /// The type of decoding errors.
    type Error: From<io::Error>;

    /// Decode an item from the src Buffer into an item
    fn decode(&mut self, src: &mut Buf) -> Result<Option<Self::Item>, Self::Error>;

    /// Called when the input stream reaches EOF, signaling a last attempt to decode
    ///
    /// # Notes
    ///
    /// The default implementation of this method invokes the `Decoder::decode` method.
    fn decode_eof(&mut self, src: &mut Buf) -> Result<Option<Self::Item>, Self::Error> {
        self.decode(src)
    }
}

impl<Buf, Io, Codec> Decoder<Buf> for Fuse<Io, Codec>
where
    Buf: Buffer,
    Codec: Decoder<Buf>,
{
    type Item = Codec::Item;
    type Error = Codec::Error;

    fn decode(&mut self, src: &mut Buf) -> Result<Option<Self::Item>, Self::Error> {
        self.codec.decode(src)
    }

    fn decode_eof(&mut self, src: &mut Buf) -> Result<Option<Self::Item>, Self::Error> {
        self.codec.decode_eof(src)
    }
}

impl<Buf, T> Decoder<Buf> for FramedWriteImpl<T, Buf>
where
    Buf: Buffer,
    T: Decoder<Buf>,
{
    type Item = T::Item;
    type Error = T::Error;

    fn decode(&mut self, src: &mut Buf) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src)
    }

    fn decode_eof(&mut self, src: &mut Buf) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode_eof(src)
    }
}
