use dencode::{FramedRead, LinesCodec};
use futures::{executor, io::Cursor, TryStreamExt};

#[test]
fn it_works() {
    let buf = "Hello\nWorld\nError".to_owned();
    let cur = Cursor::new(buf);

    let mut framed = FramedRead::new(cur, LinesCodec {});
    let next = executor::block_on(framed.try_next()).unwrap().unwrap();
    assert_eq!(next, "Hello\n");
    let next = executor::block_on(framed.try_next()).unwrap().unwrap();
    assert_eq!(next, "World\n");

    assert!(executor::block_on(framed.try_next()).is_err());
}
