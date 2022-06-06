#![deny(
    clippy::all,
    // missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
//! Utilities for encoding and decoding frames using `async/await`.
//!
//! Contains adapters to go from streams of bytes, [`AsyncRead`](futures-util::io::AsyncRead)
//! and [`AsyncWrite`](futures-util::io::AsyncWrite), to framed streams implementing [`Sink`](futures-util::Sink) and [`Stream`](futures-util::Stream).
//! Framed streams are also known as `transports`.
//!
//! ```
//! # futures::executor::block_on(async move {
//! use dencode::{Framed, LinesCodec};
//! use futures::{io::Cursor, TryStreamExt};
//!
//! let io = Cursor::new(Vec::new());
//! let mut framed = Framed::new(io, LinesCodec {});
//!
//! while let Some(line) = framed.try_next().await? {
//!     dbg!(line);
//! }
//! # Ok::<_, std::io::Error>(())
//! # }).unwrap();
//! ```

mod buffer;
pub use buffer::Buffer;

mod decoder;
pub use decoder::Decoder;

mod encoder;
pub use encoder::Encoder;

mod framed;
pub use framed::Framed;

mod framed_read;
pub use framed_read::FramedRead;

mod framed_write;
pub use framed_write::FramedWrite;

mod fuse;

mod sink;
pub use sink::{IterSink, IterSinkExt};

#[derive(Debug)]
pub struct LinesCodec {}

impl Encoder<&str, Vec<u8>> for LinesCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: &str, dst: &mut Vec<u8>) -> Result<(), Self::Error> {
        dst.extend_from_slice(item.as_bytes());
        dst.push(b'\n');
        Ok(())
    }
}

impl Decoder<Vec<u8>> for LinesCodec {
    type Item = String;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut Vec<u8>) -> Result<Option<Self::Item>, Self::Error> {
        match src.iter().position(|b| *b == b'\n') {
            Some(pos) => {
                let buf = src.drain(..pos + 1).collect();
                String::from_utf8(buf)
                    .map(Some)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
            _ => Ok(None),
        }
    }
}
