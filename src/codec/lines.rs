use std::io::{Error, ErrorKind};

use bytes::{BufMut, BytesMut};

use crate::{Decoder, Encoder};

/// A simple `Codec` implementation that splits up data into lines.
///
/// ```rust
/// # futures::executor::block_on(async move {
/// use dencode::{FramedRead, LinesCodec};
/// use futures::stream::TryStreamExt; // for lines.try_next()
///
/// let input = "hello\nworld\nthis\nis\ndog\n".as_bytes();
/// let mut lines = FramedRead::new(input, LinesCodec {});
/// while let Some(line) = lines.try_next().await? {
///     println!("{}", line);
/// }
/// # Ok::<_, std::io::Error>(())
/// # }).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LinesCodec {}

impl Encoder<String> for LinesCodec {
    type Error = Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len());
        dst.put(item.as_bytes());
        Ok(())
    }
}

impl Decoder for LinesCodec {
    type Item = String;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match src.iter().position(|b| *b == b'\n') {
            Some(pos) => {
                let buf = src.split_to(pos + 1);
                String::from_utf8(buf.to_vec())
                    .map(Some)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))
            }
            _ => Ok(None),
        }
    }
}
