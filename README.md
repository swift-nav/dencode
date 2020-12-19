# dencode

Utilities for encoding and decoding frames with support for synchronous and asynchronous io.

Contains adapters to go from streams of bytes, `Read`/`AsyncRead` and `Write`/`AsyncWrite`, to framed iterators/streams.

[![Latest Version](https://img.shields.io/crates/v/dencode.svg)](https://crates.io/crates/dencode)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/dencode)
![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)

### Example
```rust
use dencode::{LinesCodec, Framed};

async fn main() {
    // Synchronous
    // let reader = ...
    let mut framed = Framed::new(read, LinesCodec {});

    for frame in framed {
        println!("{:?}", frame);
    }

    // Asynchronous
    // let stream = ...
    let mut framed = Framed::new(stream, LinesCodec {});

    while let Some(line) = framed.try_next().await.unwrap() {
        println!("{:?}", line);
    }
}
```

### Prior Art

- [futures-codec](https://github.com/matthunz/futures-codec) - This project was originally forked from this crate.
- [tokio-codec](https://github.com/tokio-rs/tokio/tree/master/tokio-util)
