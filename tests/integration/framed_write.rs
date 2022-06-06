use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use dencode::{FramedWrite, IterSinkExt, LinesCodec};
use futures::{
    executor,
    io::{AsyncWrite, Cursor},
    sink::SinkExt,
    stream,
    stream::StreamExt,
};

use crate::BytesCodec;

// An iterator which outputs a single zero byte up to limit times
struct ZeroBytes {
    count: usize,
    limit: usize,
}

impl Iterator for ZeroBytes {
    type Item = Bytes;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.limit {
            None
        } else {
            self.count += 1;
            Some(Bytes::from_static(b"\0"))
        }
    }
}

// An AsyncWrite which is always ready and just consumes the data
struct WriteNull {
    // number of poll_write calls
    num_poll_write: usize,

    // size of the last poll_write
    last_write_size: usize,
}

impl AsyncWrite for WriteNull {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.num_poll_write += 1;
        self.last_write_size = buf.len();
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl io::Write for WriteNull {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.num_poll_write += 1;
        self.last_write_size = buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn line_write() {
    let curs = Cursor::new(vec![0u8; 16]);
    let mut framer = FramedWrite::new(curs, LinesCodec {});
    executor::block_on(framer.send("Hello")).unwrap();
    executor::block_on(framer.send("World")).unwrap();
    let (curs, _) = framer.release();
    assert_eq!(&curs.get_ref()[0..12], b"Hello\nWorld\n");
    assert_eq!(curs.position(), 12);
}

#[test]
fn blocking_line_write() {
    let curs = std::io::Cursor::new(vec![0u8; 16]);
    let mut framer = FramedWrite::new(curs, LinesCodec {});
    framer.send("Hello").unwrap();
    framer.send("World").unwrap();
    let (curs, _) = framer.release();
    assert_eq!(&curs.get_ref()[0..12], b"Hello\nWorld\n");
    assert_eq!(curs.position(), 12);
}

#[test]
fn line_write_to_eof() {
    let mut buf = [0u8; 16];
    let curs = Cursor::new(&mut buf[..]);
    let mut framer = FramedWrite::new(curs, LinesCodec {});
    let _err = executor::block_on(framer.send("This will fill up the buffer\n")).unwrap_err();
    let (curs, _) = framer.release();
    assert_eq!(curs.position(), 16);
    assert_eq!(&curs.get_ref()[0..16], b"This will fill u");
}

#[test]
fn blocking_write_to_eof() {
    let mut buf = [0u8; 16];
    let curs = std::io::Cursor::new(&mut buf[..]);
    let mut framer = FramedWrite::new(curs, LinesCodec {});
    let _err = framer.send("This will fill up the buffer\n").unwrap_err();
    let (curs, _) = framer.release();
    assert_eq!(curs.position(), 16);
    assert_eq!(&curs.get_ref()[0..16], b"This will fill u");
}

#[test]
fn send_high_water_mark() {
    // stream will output 999 bytes, 1 at at a time, and will always be ready
    let mut stream = stream::iter(ZeroBytes {
        count: 0,
        limit: 999,
    })
    .map(Ok);

    // sink will eat whatever it receives
    let io = WriteNull {
        num_poll_write: 0,
        last_write_size: 0,
    };

    // expect two sends
    let mut framer = FramedWrite::new(io, BytesCodec {});
    framer.set_send_high_water_mark(500);
    executor::block_on(SinkExt::send_all(&mut framer, &mut stream)).unwrap();
    let (io, _) = framer.release();
    assert_eq!(io.num_poll_write, 2);
    assert_eq!(io.last_write_size, 499);
}

#[test]
fn blocking_send_high_water_mark() {
    // stream will output 999 bytes, 1 at at a time, and will always be ready
    let it = ZeroBytes {
        count: 0,
        limit: 999,
    }
    .map(Ok);

    // sink will eat whatever it receives
    let io = WriteNull {
        num_poll_write: 0,
        last_write_size: 0,
    };

    // expect two sends
    let mut framer = FramedWrite::new(io, BytesCodec {});
    framer.set_send_high_water_mark(500);
    IterSinkExt::send_all(&mut framer, it).unwrap();
    let (io, _) = framer.release();
    assert_eq!(io.num_poll_write, 2);
    assert_eq!(io.last_write_size, 499);
}
