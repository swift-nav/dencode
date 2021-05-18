use std::io::Error;

use bytes::BytesMut;

use super::fuse::Fuse;

/// Encoding of messages as bytes, for use with `FramedWrite`.
pub trait Encoder<Item> {
    /// The type of encoding errors.
    type Error: From<Error>;

    /// Encodes an item into the `BytesMut` provided by dst.
    fn encode(&mut self, item: Item, dst: &mut BytesMut) -> Result<(), Self::Error>;
}

impl<T, Item, U: Encoder<Item>> Encoder<Item> for Fuse<T, U> {
    type Error = U::Error;

    fn encode(&mut self, item: Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.codec.encode(item, dst)
    }
}
