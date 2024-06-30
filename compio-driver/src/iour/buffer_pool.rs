use std::ops::{Deref, DerefMut};

use io_uring_buf_ring::IoUringBufRing;

pub struct BufferPool {
    buf_ring: IoUringBufRing<Vec<u8>>,
}

impl BufferPool {
    pub(super) fn new(buf_ring: IoUringBufRing<Vec<u8>>) -> Self {
        Self { buf_ring }
    }

    pub(super) fn buffer_group(&self) -> u16 {
        self.buf_ring.buffer_group()
    }

    pub(super) fn into_inner(self) -> IoUringBufRing<Vec<u8>> {
        self.buf_ring
    }

    pub(super) unsafe fn pick_buffer(
        &self,
        buffer_id: u16,
        available_len: usize,
    ) -> Option<BorrowedBuffer> {
        self.buf_ring
            .get_buf(buffer_id, available_len)
            .map(BorrowedBuffer)
    }
}

pub struct BorrowedBuffer<'a>(io_uring_buf_ring::BorrowedBuffer<'a, Vec<u8>>);

impl Deref for BorrowedBuffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for BorrowedBuffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}
