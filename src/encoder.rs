use std::io;

use super::fuse::Fuse;
use crate::Buffer;

/// Encoding of messages as bytes, for use with `FramedWrite`.
pub trait Encoder<Item, Buf>
where
    Buf: Buffer,
{
    /// The type of encoding errors.
    type Error: From<io::Error>;

    /// Encodes an item into the `BytesMut` provided by dst.
    fn encode(&mut self, item: Item, dst: &mut Buf) -> Result<(), Self::Error>;
}

impl<T, Item, Buf, U> Encoder<Item, Buf> for Fuse<T, U>
where
    Buf: Buffer,
    U: Encoder<Item, Buf>,
{
    type Error = U::Error;

    fn encode(&mut self, item: Item, dst: &mut Buf) -> Result<(), Self::Error> {
        self.codec.encode(item, dst)
    }
}
