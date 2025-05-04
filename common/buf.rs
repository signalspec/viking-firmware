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

pub struct Reader<'a> {
    buf: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Reader<'a> {
        Reader { buf }
    }
    
    pub fn take_first(&mut self) -> Option<u8> {
        let (first, rem) = self.buf.split_first()?;
        self.buf = rem;
        Some(*first)
    }

    pub fn take_len(&mut self) -> Option<&'a [u8]> {
        let len = self.take_first()? as usize;
        let (first, rem) = self.buf.split_at(len);
        self.buf = rem;
        Some(first)
    }

    pub fn take_u16(&mut self) -> Option<u16> {
        let (first, rem) = self.buf.split_at_checked(2)?;
        self.buf = rem;
        Some(u16::from_le_bytes([first[0], first[1]]))
    }
}

