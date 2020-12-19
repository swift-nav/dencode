#![deny(
    clippy::all,
    missing_docs,
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

pub use bytes::{Buf, BufMut, Bytes, BytesMut};

mod codec;
pub use codec::{bytes::BytesCodec, lines::LinesCodec};

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

mod sink;
pub use sink::{IterSink, IterSinkExt};

mod fuse;
