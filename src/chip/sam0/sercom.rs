use zeptos::{executor::{Interrupt, TaskOnly}, samd::pac::{interrupt, sercom0::RegisterBlock, SERCOM0, SERCOM1}};

pub trait Sercom {
    const NUM: usize;
}
pub struct Sercom0;

impl Sercom for Sercom0 {
    const NUM: usize = 0;
}

pub(crate) struct DynSercom(pub(crate) usize);

impl DynSercom {
    pub(crate) fn regs(&self) -> &RegisterBlock {
        unsafe { &*(SERCOM0::PTR.byte_offset((SERCOM1::PTR as usize - SERCOM0::PTR as usize) as isize * self.0 as isize) ) }
    }

    pub(crate) fn notify(&self) -> &Interrupt {
        unsafe { INT.get_unchecked() }
    }
}

static INT: TaskOnly<Interrupt> = unsafe { TaskOnly::new(Interrupt::new()) };

#[interrupt]
fn SERCOM0() {
    let sercom = unsafe { SERCOM0::steal() };
    unsafe { INT.get_unchecked().notify() };
}
