pub trait Buffer {
    fn with_capacity(capacity: usize) -> Self;

    fn extend_from_slice(&mut self, src: &[u8]);

    fn is_empty(&self) -> bool;

    fn advance(&mut self, cnt: usize);

    fn len(&self) -> usize;

    fn as_slice(&self) -> &[u8];
}

impl Buffer for Vec<u8> {
    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn extend_from_slice(&mut self, src: &[u8]) {
        self.extend_from_slice(src)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn advance(&mut self, cnt: usize) {
        self.drain(..cnt);
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn as_slice(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(feature = "bytes")]
mod bytes_impl {
    use bytes::{Buf, BytesMut};

    use super::Buffer;

    impl Buffer for BytesMut {
        fn with_capacity(capacity: usize) -> Self {
            BytesMut::with_capacity(capacity)
        }

        fn extend_from_slice(&mut self, src: &[u8]) {
            self.extend_from_slice(src)
        }

        fn is_empty(&self) -> bool {
            self.is_empty()
        }

        fn advance(&mut self, cnt: usize) {
            Buf::advance(self, cnt)
        }

        fn len(&self) -> usize {
            self.len()
        }

        fn as_slice(&self) -> &[u8] {
            &self[..]
        }
    }
}

#[cfg(feature = "bitvec")]
mod bitvec_impl {
    use bitvec::prelude::*;

    use super::Buffer;

    impl<O> Buffer for BitVec<u8, O>
    where
        O: BitOrder,
    {
        fn with_capacity(capacity: usize) -> Self {
            BitVec::with_capacity(capacity)
        }

        fn extend_from_slice(&mut self, src: &[u8]) {
            // self.extend_from_bitslice::<_, LocalBits>(BitSlice::from_slice(src))
            self.extend_from_bitslice::<_, O>(BitSlice::from_slice(src))
        }

        fn is_empty(&self) -> bool {
            self.is_empty()
        }

        fn advance(&mut self, cnt: usize) {
            self.drain(..cnt);
        }

        fn len(&self) -> usize {
            self.len()
        }

        fn as_slice(&self) -> &[u8] {
            self.as_raw_slice()
        }
    }
}
