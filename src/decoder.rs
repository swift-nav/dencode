use std::io::Error;

use bytes::BytesMut;

use super::{framed_write::FramedWriteImpl, fuse::Fuse};

/// Decoding of frames via buffers, for use with `FramedRead`.
pub trait Decoder {
    /// The type of items returned by `decode`
    type Item;
    /// The type of decoding errors.
    type Error: From<Error>;

    /// Decode an item from the src `BytesMut` into an item
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error>;

    /// Called when the input stream reaches EOF, signaling a last attempt to decode
    ///
    /// # Notes
    ///
    /// The default implementation of this method invokes the `Decoder::decode` method.
    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode(src)
    }
}

impl<T, U: Decoder> Decoder for Fuse<T, U> {
    type Item = U::Item;
    type Error = U::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.codec.decode(src)
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.codec.decode_eof(src)
    }
}

impl<T: Decoder> Decoder for FramedWriteImpl<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode(src)
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.inner.decode_eof(src)
    }
}
