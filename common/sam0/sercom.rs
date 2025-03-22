use zeptos::{executor::{Interrupt, TaskOnly}, samd::pac::{interrupt, sercom0::RegisterBlock, SERCOM0, SERCOM1}};

pub trait Sercom {
    const NUM: usize;
}
pub struct Sercom0;

impl Sercom for Sercom0 {
    const NUM: usize = 0;
}

pub struct Sercom1;

impl Sercom for Sercom1 {
    const NUM: usize = 1;
}

pub struct Sercom2;

impl Sercom for Sercom2 {
    const NUM: usize = 1;
}

pub(crate) struct DynSercom(pub(crate) usize);

impl DynSercom {
    pub(crate) fn regs(&self) -> &RegisterBlock {
        unsafe { &*(SERCOM0::PTR.byte_offset((SERCOM1::PTR as usize - SERCOM0::PTR as usize) as isize * self.0 as isize) ) }
    }

    pub(crate) fn notify(&self) -> &Interrupt {
        unsafe { &INT.get_unchecked()[self.0] }
    }
}

static INT: TaskOnly<[Interrupt; 3]> = unsafe { TaskOnly::new([const { Interrupt::new() }; 3]) };

#[interrupt]
fn SERCOM0() {
    unsafe { INT.get_unchecked()[0].notify() };
}

#[interrupt]
fn SERCOM1() {
    unsafe { INT.get_unchecked()[1].notify() };
}

#[interrupt]
fn SERCOM2() {
    unsafe { INT.get_unchecked()[2].notify() };
}

