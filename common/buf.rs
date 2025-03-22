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

    pub fn put(&mut self, b: u8) -> Result<(), ()> {
        let next = self.buf.get_mut(self.offset).ok_or(())?;
        *next = b;
        self.offset += 1;
        Ok(())
    }
}

pub fn take_first<'a>(buf: &mut &'a [u8]) -> Option<u8> {
    let (first, rem) = buf.split_first()?;
    *buf = rem;
    Some(*first)
}

pub fn take_len<'a>(buf: &mut &'a [u8]) -> Option<&'a [u8]> {
    let len = take_first(buf)? as usize;
    let s = buf.get(..len)?;
    *buf = &buf[len..];
    Some(s)
}