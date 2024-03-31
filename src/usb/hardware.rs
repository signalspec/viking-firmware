use core::sync::atomic::{AtomicPtr, AtomicU16, AtomicU32, AtomicU8, Ordering};

use atsamd_hal::pac::usb::{device::{EPCFG, EPINTENCLR, EPINTENSET, EPINTFLAG, EPSTATUS, EPSTATUSCLR, EPSTATUSSET}, DEVICE};
use defmt::{ assert, debug_assert };

#[inline(always)]
pub fn ep_regs(regs: &DEVICE, ep: u8) -> &DEVICE_EP {
    assert!(ep < 8);
    unsafe {
        &*(regs as *const DEVICE)
            .cast::<DEVICE_EP>()
            .byte_offset(0x100 + 0x20 * (ep as isize))
    }
}

#[allow(non_camel_case_types)]
pub struct DEVICE_EP {
    #[doc = "+0x00 - DEVICE End Point Configuration"]
    pub epcfg: EPCFG,
    _reserved1: [u8; 0x03],
    #[doc = "+0x04 - DEVICE End Point Pipe Status Clear"]
    pub epstatusclr: EPSTATUSCLR,
    #[doc = "+0x05 - DEVICE End Point Pipe Status Set"]
    pub epstatusset: EPSTATUSSET,
    #[doc = "+0x06 - DEVICE End Point Pipe Status"]
    pub epstatus: EPSTATUS,
    #[doc = "+0x07 - DEVICE End Point Interrupt Flag"]
    pub epintflag: EPINTFLAG,
    #[doc = "+0x08 - DEVICE End Point Interrupt Clear Flag"]
    pub epintenclr: EPINTENCLR,
    #[doc = "+0x09 - DEVICE End Point Interrupt Set Flag"]
    pub epintenset: EPINTENSET,
}


#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PacketSize {
    Size8 = 0,
    Size16 = 1,
    Size32 = 2,
    Size64 = 3,
    Size128 = 4,
    Size256 = 5,
    Size512 = 6,
    Size1023 = 7,
}

impl PacketSize {
    const fn value(self) -> usize {
        match self {
            PacketSize::Size8 => 8,
            PacketSize::Size16 => 16,
            PacketSize::Size32 => 32,
            PacketSize::Size64 => 64,
            PacketSize::Size128 => 128,
            PacketSize::Size256 => 256,
            PacketSize::Size512 => 512,
            PacketSize::Size1023 => 1023,
        }
    }
}

/// Per-endpoint data descriptor accessed by the hardware but stored in RAM.
/// 
/// We can model the DMA access as if it were another thread, so use
/// atomic types, but all synchronization is done via registers,
/// so can be relaxed.
#[repr(C, align(4))]
pub struct EndpointBank {
    addr: AtomicPtr<u8>,
    pcksize: AtomicU32,
    extreg: AtomicU16,
    status_bk: AtomicU8,
    _reserved: [u8; 5],
}

impl EndpointBank {    
    pub fn prepare_out(
        &self,
        packet_size: PacketSize,
        ptr: *mut u8,
        len: usize,
    ) {
        debug_assert!(len % (1<<(packet_size as u8 + 3)) == 0);
        debug_assert!(len < (1<<14));
        self.addr.store(ptr, Ordering::Relaxed);
        self.pcksize.store(
        (len as u32) << 14 // MULTI_PACKET_SIZE
            | (packet_size as u8 as u32) << 28, // SIZE
            Ordering::Relaxed
        );
    }

    pub fn out_len(&self) -> usize {
        let pcksize = self.pcksize.load(Ordering::Relaxed);
        (pcksize & ((1 << 14) - 1)) as usize
    }

    pub fn prepare_in(
        &self,
        packet_size: PacketSize,
        ptr: *mut u8,
        len: usize,
        zlp: bool,
    ) {
        debug_assert!(len < (1<<14));
        self.addr.store(ptr, Ordering::Relaxed);
        self.pcksize.store(
            (len as u32) // BYTE_COUNT
            | (packet_size as u8 as u32) << 28 // SIZE
            | (zlp as u32) << 31, // AUTO_ZLP
            Ordering::Relaxed
        );

    }
}