use core::{marker::PhantomData, slice};

use crate::common::ErrorByte;
use viking_protocol::errors::ERR_RESPONSE_FULL;

pub struct Writer<'a> {
    start: *mut u8,
    pos: *mut u8,
    end: *mut u8,
    _phantom: PhantomData<&'a mut [u8]>,
}

impl<'a> Writer<'a> {
    pub fn new(buf: &'a mut [u8], offset: usize) -> Writer<'a> {
        Writer {
            start: buf.as_mut_ptr(),
            pos: unsafe { buf.as_mut_ptr().add(offset) },
            end: unsafe { buf.as_mut_ptr().add(buf.len()) },
            _phantom: PhantomData,
        }
    }

    pub fn offset(&self) -> usize {
        unsafe { self.pos.offset_from_unsigned(self.start) }
    }

    pub fn put(&mut self, b: u8) -> Result<(), ErrorByte> {
        if self.pos < self.end {
            unsafe { *self.pos = b };
            self.pos = unsafe { self.pos.add(1) };
            Ok(())
        } else {
            Err(ERR_RESPONSE_FULL)
        }
    }

    pub fn reserve(&mut self) -> Result<&'a mut u8, ()> {
        if self.pos < self.end {
            let r = unsafe { &mut *self.pos };
            self.pos = unsafe { self.pos.add(1) };
            Ok(r)
        } else {
            Err(())
        }
    }

    pub fn remaining(&self) -> usize {
        unsafe { self.end.offset_from_unsigned(self.pos) }
    }

    pub fn reserve_buf(&mut self, n: usize) -> Result<&'a mut [u8], ()> {
        if self.remaining() >= n {
            let r = unsafe { slice::from_raw_parts_mut(self.pos, n) };
            self.pos = unsafe { self.pos.add(n) };
            Ok(r)
        } else {
            Err(())
        }
    }
}

pub struct Reader<'a> {
    ptr: *const u8,
    end: *const u8,
    _phantom: PhantomData<&'a [u8]>,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Reader<'a> {
        Reader {
            ptr: buf.as_ptr(),
            end: unsafe { buf.as_ptr().add(buf.len()) },
            _phantom: PhantomData,
        }
    }

    pub fn remaining(&self) -> usize {
        unsafe { self.end.offset_from_unsigned(self.ptr) }
    }

    pub fn take_first(&mut self) -> Option<u8> {
        if self.ptr < self.end {
            let b = unsafe { *self.ptr };
            self.ptr = unsafe { self.ptr.add(1) };
            Some(b)
        } else {
            None
        }
    }

    pub fn take_n<const N: usize>(&mut self) -> Option<&'a [u8; N]> {
        if self.remaining() >= N {
            let r = unsafe {&*(self.ptr as *const [u8; N])};
            self.ptr = unsafe { self.ptr.add(N) };
            Some(r)
        } else {
            None
        }
    }

    pub fn take_len(&mut self) -> Option<&'a [u8]> {
        let len = self.take_first()? as usize;
        if self.remaining() >= len {
            let r = unsafe {slice::from_raw_parts(self.ptr, len)};
            self.ptr = unsafe { self.ptr.add(len) };
            Some(r)
        } else {
            None
        }
    }

    pub fn take_u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes([self.take_first()?, self.take_first()?]))
    }
}
