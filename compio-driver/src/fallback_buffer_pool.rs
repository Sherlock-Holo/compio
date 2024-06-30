use std::{
    cell::RefCell,
    collections::VecDeque,
    fmt::{Debug, Formatter},
    mem,
    ops::{Deref, DerefMut},
};

pub struct BufferPool {
    buffers: RefCell<VecDeque<Vec<u8>>>,
}

impl Debug for BufferPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferPool").finish_non_exhaustive()
    }
}

impl BufferPool {
    pub(crate) fn new(buffer_size: usize, buffer_len: usize) -> Self {
        let buffers = (0..buffer_len).map(|_| vec![0; buffer_size]).collect();

        Self {
            buffers: RefCell::new(buffers),
        }
    }

    pub(crate) fn pick_buffer(&self) -> Option<BorrowedBuffer> {
        self.buffers
            .borrow_mut()
            .pop_front()
            .map(|buffer| BorrowedBuffer {
                buffer,
                len: 0,
                pool: self,
            })
    }

    fn add_buffer(&self, buffer: &mut BorrowedBuffer) {
        self.buffers
            .borrow_mut()
            .push_back(mem::take(&mut buffer.buffer));
    }
}

pub struct BorrowedBuffer<'a> {
    buffer: Vec<u8>,
    len: usize,
    pool: &'a BufferPool,
}

impl Debug for BorrowedBuffer<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BorrowedBuffer").finish_non_exhaustive()
    }
}

impl BorrowedBuffer<'_> {
    pub(super) fn set_len(&mut self, len: usize) {
        self.len = len;
    }
}

impl Deref for BorrowedBuffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffer[..self.len]
    }
}

impl DerefMut for BorrowedBuffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer[..self.len]
    }
}

impl Drop for BorrowedBuffer<'_> {
    fn drop(&mut self) {
        self.pool.add_buffer(self)
    }
}
