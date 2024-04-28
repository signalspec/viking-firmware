/// Largely copied from atsamd-hal under Apache-2.0 OR MIT.
use core::marker::PhantomData;

use atsamd_hal::{pac::{port::{
    CTRL, DIR, DIRCLR, DIRSET, DIRTGL, IN, OUT, OUTCLR, OUTSET, OUTTGL, PINCFG0_ as PINCFG,
    PMUX0_ as PMUX, WRCONFIG,
}, PORT, PORT_IOBUS}, gpio::{DynPinId, DynGroup, Pin, PinMode}};

pub use atsamd_hal::gpio::PinId;

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

pub struct IoPin<P: PinId>(PhantomData<P>);

impl<P: PinId> IoPin<P> {
    #[inline]
    fn id() -> DynPinId {
        P::DYN
    }

    #[inline]
    fn group_offset() -> usize {
        match Self::id().group {
            DynGroup::A => 0,
            DynGroup::B => 1,
            //DynGroup::C => 2,
            //DynGroup::D => 3,
        }
    }

    #[inline]
    fn group() -> &'static GROUP {
        const GROUPS: *const GROUP = PORT::ptr() as *const _;
        
        // Safety: It is safe to create shared references to each PAC register
        // or register block, because all registers are wrapped in
        // `UnsafeCell`s. We should never create unique references to the
        // registers, to prevent any risk of UB.
        unsafe { &*GROUPS.add(Self::group_offset()) }
    }

    #[inline]
    fn group_iobus() -> &'static GROUP {
        const GROUPS: *const GROUP = PORT_IOBUS::ptr() as *const _;
        
        // Safety: It is safe to create shared references to each PAC register
        // or register block, because all registers are wrapped in
        // `UnsafeCell`s. We should never create unique references to the
        // registers, to prevent any risk of UB.
        unsafe { &*GROUPS.add(Self::group_offset()) }
    }

    #[inline]
    fn mask_32() -> u32 {
        1 << P::DYN.num
    }

    pub fn enable_sampling(&self) {
        unsafe {
            Self::group().ctrl.write(|w| w.bits(0xffffffff))
        }
    }

    #[inline]
    pub fn pincfg() -> &'static PINCFG {
        &Self::group().pincfg[Self::id().num as usize]
    }

    #[inline]    
    pub fn read() -> bool {
        let mask = Self::mask_32();
        Self::group_iobus().in_.read().bits() & mask != 0
    }

    #[inline]
    pub fn read_out() -> bool {
        let mask = Self::mask_32();
        Self::group_iobus().out.read().bits() & mask != 0
    }

    #[inline]
    pub fn read_dir() -> bool {
        let mask = Self::mask_32();
        Self::group_iobus().dir.read().bits() & mask != 0
    }

    #[inline]
    pub fn outset() {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            Self::group_iobus().outset.write(|w| w.bits(Self::mask_32()));
        }
    }

    #[inline]
    pub fn outclr() {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            Self::group_iobus().outclr.write(|w| w.bits(Self::mask_32()));
        }
    }

    #[inline]
    pub fn outtgl() {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            Self::group_iobus().outtgl.write(|w| w.bits(Self::mask_32()));
        }
    }


    #[inline]
    pub fn dirset() {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            Self::group_iobus().dirset.write(|w| w.bits(Self::mask_32()));
        }
    }

    #[inline]
    pub fn dirclr() {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            Self::group_iobus().dirclr.write(|w| w.bits(Self::mask_32()));
        }
    }

    #[inline]
    pub fn dirtgl() {
        //SAFETY: these are "mask" registers, and we only write the bit for this pin ID
        unsafe {
            Self::group_iobus().dirtgl.write(|w| w.bits(Self::mask_32()));
        }
    }
}
