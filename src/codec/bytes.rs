use std::io::Error;

use bytes::{Bytes, BytesMut};

use crate::{Decoder, Encoder};

/// A simple codec that ships bytes around
///
/// # Example
///
///  ```
/// # futures::executor::block_on(async move {
/// use bytes::Bytes;
/// use futures::{SinkExt, TryStreamExt};
/// use futures::io::Cursor;
/// use dencode::{BytesCodec, Framed};
///
/// let mut buf = vec![];
/// // Cursor implements AsyncRead and AsyncWrite
/// let cur = Cursor::new(&mut buf);
/// let mut framed = Framed::new(cur, BytesCodec {});
///
/// framed.send(Bytes::from("Hello World!")).await?;
///
/// while let Some(bytes) = framed.try_next().await? {
///     dbg!(bytes);
/// }
/// # Ok::<_, std::io::Error>(())
/// # }).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BytesCodec {}

impl Encoder<Bytes> for BytesCodec {
    type Error = Error;

    fn encode(&mut self, src: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&src);
        Ok(())
    }
}

impl Decoder for BytesCodec {
    type Item = Bytes;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        if len > 0 {
            Ok(Some(src.split_to(len).freeze()))
        } else {
            Ok(None)
        }
    }
}
