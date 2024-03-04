/// Largely copied from atsamd-hal under Apache-2.0 OR MIT.
use core::marker::PhantomData;

use atsamd_hal::{pac::{port::{
    CTRL, DIR, DIRCLR, DIRSET, DIRTGL, IN, OUT, OUTCLR, OUTSET, OUTTGL, PINCFG0_ as PINCFG,
    PMUX0_ as PMUX, WRCONFIG,
}, PORT, PORT_IOBUS}, gpio::{PinId, DynPinId, DynGroup, Pin, PinMode}};

/// The [`PORT`] register block
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
pub(super) struct GROUP {
    dir: DIR,
    dirclr: DIRCLR,
    dirset: DIRSET,
    dirtgl: DIRTGL,
    out: OUT,
    outclr: OUTCLR,
    outset: OUTSET,
    outtgl: OUTTGL,
    in_: IN,
    ctrl: CTRL,
    wrconfig: WRCONFIG,
    _padding1: [u8; 4],
    pmux: [PMUX; 16],
    pincfg: [PINCFG; 32],
    _padding2: [u8; 32],
}

pub struct IoPin<P: PinId> {
    _id: PhantomData<P>
}

impl<P: PinId, M: PinMode> From<Pin<P, M>> for IoPin<P> {
    fn from(pin: Pin<P, M>) -> Self {
        IoPin::new(pin)
    }
}

impl<P: PinId> IoPin<P> {
    // SAFETY: consumes the pin, guaranteeing exclusive access
    pub fn new<M: PinMode>(p: Pin<P, M>) -> IoPin<P> {
        IoPin{ _id: PhantomData }
    }

    #[inline]
    fn id(&self) -> DynPinId {
        P::DYN
    }

    #[inline]
    fn group_offset(&self) -> usize {
        match self.id().group {
            DynGroup::A => 0,
            //DynGroup::B => 1,
            //DynGroup::C => 2,
            //DynGroup::D => 3,
        }
    }

    #[inline]
    fn group(&self) -> &GROUP {
        const GROUPS: *const GROUP = PORT::ptr() as *const _;
        
        // Safety: It is safe to create shared references to each PAC register
        // or register block, because all registers are wrapped in
        // `UnsafeCell`s. We should never create unique references to the
        // registers, to prevent any risk of UB.
        unsafe { &*GROUPS.add(self.group_offset()) }
    }

    #[inline]
    fn group_iobus(&self) -> &GROUP {
        const GROUPS: *const GROUP = PORT_IOBUS::ptr() as *const _;
        
        // Safety: It is safe to create shared references to each PAC register
        // or register block, because all registers are wrapped in
        // `UnsafeCell`s. We should never create unique references to the
        // registers, to prevent any risk of UB.
        unsafe { &*GROUPS.add(self.group_offset()) }
    }

    #[inline]
    fn mask_32(&self) -> u32 {
        1 << self.id().num
    }

    pub fn enable_sampling(&mut self) {
        unsafe {
            self.group().ctrl.write(|w| w.bits(0xffffffff))
        }
    }

    #[inline]
    pub fn pincfg(&self) -> &PINCFG {
        &self.group().pincfg[self.id().num as usize]
    }

    #[inline]    
    pub fn read(&self) -> bool {
        let mask = self.mask_32();
        self.group_iobus().in_.read().bits() & mask != 0
    }

    #[inline]    
    pub fn read_out(&self) -> bool {
        let mask = self.mask_32();
        self.group_iobus().out.read().bits() & mask != 0
    }

    #[inline]    
    pub fn read_dir(&self) -> bool {
        let mask = self.mask_32();
        self.group_iobus().dir.read().bits() & mask != 0
    }

    #[inline]
    pub fn outset(&mut self) {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            self.group_iobus().outset.write(|w| w.bits(self.mask_32()));
        }
    }

    #[inline]
    pub fn outclr(&mut self) {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            self.group_iobus().outclr.write(|w| w.bits(self.mask_32()));
        }
    }

    #[inline]
    pub fn outtgl(&mut self) {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            self.group_iobus().outtgl.write(|w| w.bits(self.mask_32()));
        }
    }


    #[inline]
    pub fn dirset(&mut self) {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            self.group_iobus().dirset.write(|w| w.bits(self.mask_32()));
        }
    }

    #[inline]
    pub fn dirclr(&mut self) {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            self.group_iobus().dirclr.write(|w| w.bits(self.mask_32()));
        }
    }

    #[inline]
    pub fn dirtgl(&mut self) {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            self.group().dirtgl.write(|w| w.bits(self.mask_32()));
        }
    }
}
