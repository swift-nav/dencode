mod framed;
mod framed_read;
mod framed_write;

use std::io;

use bytes::{Bytes, BytesMut};
use dencode::{Decoder, Encoder};

pub struct BytesCodec {}

impl Encoder<Bytes, BytesMut> for BytesCodec {
    type Error = io::Error;

    fn encode(&mut self, src: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&src);
        Ok(())
    }
}

impl Decoder<BytesMut> for BytesCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        if len > 0 {
            Ok(Some(src.split_to(len).freeze()))
        } else {
            Ok(None)
        }
    }
}
