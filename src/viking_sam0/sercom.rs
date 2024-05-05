use atsamd_hal::pac::{sercom0::RegisterBlock, Interrupt, SERCOM0, SERCOM1, interrupt};
use lilos::exec::Notify;

const NEW_NOTIFY: Notify = Notify::new();
static IRQ: [Notify; 6] = [NEW_NOTIFY; 6];

pub(crate) struct DynSercom(pub(crate) usize);

impl DynSercom {
    pub(crate) fn regs(&self) -> &RegisterBlock {
        unsafe { &*(SERCOM0::PTR.byte_offset((SERCOM1::PTR as usize - SERCOM0::PTR as usize) as isize * self.0 as isize) ) }
    }

    pub(crate) fn notify(&self) -> &Notify {
        &IRQ[self.0]
    }
}

#[interrupt]
fn SERCOM0() {
    let sercom = unsafe { SERCOM0::steal() };
    sercom.i2cm().intenclr.write(|w| {
        unsafe { w.bits(sercom.i2cm().intflag.read().bits()) }
    });
    IRQ[0].notify();
}
