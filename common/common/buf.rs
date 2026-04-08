use core::{marker::PhantomData, slice};

use crate::common::ErrorByte;
use viking_protocol::errors::ERR_RESPONSE_FULL;

pub struct Writer<'a> {
    offset: usize,
    buf: &'a mut [u8],
}

impl<'a> Writer<'a> {
    pub fn new(buf: &'a mut [u8], offset: usize) -> Writer<'a> {
        Writer { buf, offset }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn put(&mut self, b: u8) -> Result<(), ErrorByte> {
        let next = self.buf.get_mut(self.offset).ok_or(ERR_RESPONSE_FULL)?;
        *next = b;
        self.offset += 1;
        Ok(())
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
