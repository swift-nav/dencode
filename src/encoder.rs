use std::io;

use super::fuse::Fuse;
use crate::Buffer;

/// Encoding of messages as bytes, for use with `FramedWrite`.
pub trait Encoder<Buf, Item>
where
    Buf: Buffer,
{
    /// The type of encoding errors.
    type Error: From<io::Error>;

    /// Encodes an item into the `BytesMut` provided by dst.
    fn encode(&mut self, item: Item, dst: &mut Buf) -> Result<(), Self::Error>;
}

impl<Io, Codec, Buf, Item> Encoder<Buf, Item> for Fuse<Io, Codec>
where
    Buf: Buffer,
    Codec: Encoder<Buf, Item>,
{
    type Error = Codec::Error;

    fn encode(&mut self, item: Item, dst: &mut Buf) -> Result<(), Self::Error> {
        self.codec.encode(item, dst)
    }
}
