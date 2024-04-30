mod hardware;
pub mod descriptors;

use core::{convert::Infallible, future::Future, marker::PhantomData, mem, ops::{Deref, DerefMut}, pin::Pin, sync::atomic::{compiler_fence, Ordering}, task::{Context, Poll}};
use core::pin::pin;

use atsamd_hal::{calibration::{usb_transn_cal, usb_transp_cal, usb_trim_cal}, clock::UsbClock, gpio::{AlternateG, AnyPin, PA24, PA25}, pac::{usb::DEVICE, Interrupt, USB, interrupt}};
use defmt::{debug, Format, write};
use lilos::{exec::Notify, util::FutureExt as _};
use pin_project_lite::pin_project;
use futures_util::{future::FusedFuture, select_biased, FutureExt};

use self::{descriptors::StringDecriptor, hardware::{ep_regs, EndpointBank, PacketSize, DEVICE_EP}};

static EP_RAM: [[EndpointBank; 2]; 8] = unsafe { mem::zeroed() };


#[repr(C, align(4))]
pub struct UsbBuffer<const SIZE: usize>([u8; SIZE]);

impl<const SIZE: usize> UsbBuffer<SIZE> {
    pub const fn new() -> Self {
        UsbBuffer([0; SIZE])
    }
}

impl<const SIZE: usize> Deref for UsbBuffer<SIZE> {
    type Target = [u8; SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const SIZE: usize> DerefMut for UsbBuffer<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

//static mut SETUP_PACKET: UsbBuffer<10> = UsbBuffer::new();
static mut CONTROL_BUF: UsbBuffer<64> = UsbBuffer::new();

enum Event {
    Reset
}

/// Specification defining the request.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Format)]
#[repr(u8)]
pub enum ControlType {
    /// Request defined by the USB standard.
    Standard = 0,

    /// Request defined by the standard USB class specification.
    Class = 1,

    /// Non-standard request.
    Vendor = 2,
}

/// Entity targeted by the request.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Format)]
#[repr(u8)]
pub enum Recipient {
    /// Request made to device as a whole.
    Device = 0,

    /// Request made to specific interface.
    Interface = 1,

    /// Request made to specific endpoint.
    Endpoint = 2,

    /// Other request.
    Other = 3,
}

pub struct ControlIn<'a> {
    usb: &'a Usb,
    length: u16,
}

impl<'a> ControlIn<'a> {
    pub fn reject(self) {
        debug!("reject in request");
        self.usb.stall_ep0();
    }

    pub async fn respond(self, data: &[u8]) {
        debug!("accepting in request with {} bytes", data.len());
        
        // Limit response size to host's request size
        let is_full = data.len() >= self.length as usize;
        let mut data = &data[..data.len().min(self.length as usize)];

        loop {
            // We want to be able to send an arbitrary slice, which may not be
            // correctly aligned or in RAM (USB DMA can't read from flash), so copy
            // a packet at a time to CONTROL_BUF
            let buf = unsafe { &mut CONTROL_BUF.0 };

            let (pkt, remaining) = data.split_at(data.len().min(buf.len()));
            
            buf[..pkt.len()].copy_from_slice(pkt);
            transfer_in(0x80, self.usb.ep(0x80), self.usb.ep_ram(0x80), PacketSize::Size64, buf.as_ptr(), pkt.len(), false).await;

            if pkt.len() < 64 || (remaining.len() == 0 && is_full) {
                break;
            }

            data = remaining;
        }

        debug!("data phase complete");
        let buf = unsafe { &mut CONTROL_BUF.0 };
        transfer_out(0, self.usb.ep(0x0), self.usb.ep_ram(0x0), PacketSize::Size64, buf.as_mut_ptr(), 64).await;
        debug!("status phase complete");
    }
}
pub struct ControlOut<'a> {
    usb: &'a Usb,
    length: u16,
}

impl<'a> ControlOut<'a> {
    pub fn reject(self) {
        debug!("reject out request");
        self.usb.stall_ep0();
    }

    pub fn len(&self) -> usize {
        self.length as usize
    }

    async fn accept_internal(&self) {
        debug!("accept out request");
        let buf = unsafe { &mut CONTROL_BUF.0 };
        //self.usb.transfer_out(0, PacketSize::Size64, buf.as_mut_ptr(), 64).await;
        //debug!("data stage complete");
        transfer_in(0x80, self.usb.ep(0x80), self.usb.ep_ram(0x80), PacketSize::Size64, buf.as_ptr(), 0, false).await;
        debug!("status stage complete");
    }

    pub async fn accept(self) {
        self.accept_internal().await;
    }

    pub async fn accept_and_set_address(self, addr: u8) {
        self.accept_internal().await;
        self.usb.usb().dadd.write(|w| {
            w.adden().set_bit();
            w.dadd().variant(addr)
        });
    }
}


pub enum ControlData<'a> {
    In(ControlIn<'a>),
    Out(ControlOut<'a>),
}

impl<'a> Format for ControlData<'a> {
    fn format(&self, f: defmt::Formatter) {
        match self {
            ControlData::In(d) => write!(f, "{} bytes IN", d.length),
            ControlData::Out(d) => write!(f, "{} bytes OUT", d.length),
        }
    }
}

pub struct Setup<'a> {
    pub data: ControlData<'a>,

    /// Request type used for the `bmRequestType` field sent in the SETUP packet.
    #[doc(alias = "bmRequestType")]
    pub ty: ControlType,

    /// Recipient used for the `bmRequestType` field sent in the SETUP packet.
    #[doc(alias = "bmRequestType")]
    pub recipient: Recipient,

    /// `bRequest` field sent in the SETUP packet.
    #[doc(alias = "bRequest")]
    pub request: u8,

    /// `wValue` field sent in the SETUP packet.
    #[doc(alias = "wValue")]
    pub value: u16,

    /// `wIndex` field sent in the SETUP packet.
    ///
    /// For [`Recipient::Interface`] this is the interface number. For [`Recipient::Endpoint`] this is the endpoint number.
    #[doc(alias = "wIndex")]
    pub index: u16,
}

impl<'a> Setup<'a> {
    fn parse(usb: &'a Usb, packet: [u8; 8]) -> Result<Setup<'a>, ()> {
        Ok(Setup {
            recipient: match packet[0] & 0x0F {
                0 => Recipient::Device,
                1 => Recipient::Interface,
                2 => Recipient::Endpoint,
                3 => Recipient::Other, 
                _ => return Err(()),
            },
            ty: match (packet[0] >> 5) & 0x03 {
                0 => ControlType::Standard,
                1 => ControlType::Class,
                2 => ControlType::Vendor,
                _ => return Err(())
            },
            request: packet[1],
            value: u16::from_le_bytes([packet[2], packet[3]]),
            index: u16::from_le_bytes([packet[4], packet[5]]),
            data: {
                let length = u16::from_le_bytes([packet[6], packet[7]]);
                if packet[0] & 0x80 == 0 {
                    ControlData::Out(ControlOut { length, usb })
                } else {
                    ControlData::In(ControlIn { length, usb })
                }
            }
        })
    }

    pub fn reject(self) {
        match self.data {
            ControlData::In(d) => d.reject(),
            ControlData::Out(d) => d.reject(),
        }
    }
}

pub trait Handler {
    async fn handle_reset(&self) {
        debug!("usb reset");
    }

    fn get_descriptor(&self, _kind: u8, _index: u8) -> Option<&[u8]> {
        None
    }

    fn get_string_descriptor(&self, _index: u8, _lang: u16) -> Option<StringDecriptor<128>> {
        None
    }

    async fn set_configuration(&self, cfg: u8, _usb: &Usb) -> Result<(), ()> {
        if cfg == 1 { Ok(()) } else { Err(()) }
    }

    async fn set_interface(&self, intf: u8, alt: u8, _usb: &Usb) -> Result<(), ()> {
        Err(())
    }

    async fn handle_control<'a>(&self, req: Setup<'a>, usb: &Usb) {
        req.reject();
    }
}

pub struct Usb {

}

impl Usb {
    pub fn new(
        _clock: &UsbClock,
        dm_pad: impl AnyPin<Id = PA24>,
        dp_pad: impl AnyPin<Id = PA25>,
        _usb: USB,
    ) -> Self {
        dm_pad.into().into_mode::<AlternateG>();
        dp_pad.into().into_mode::<AlternateG>();

        let mut r = Usb {};
        r.enable();
        r
    }

    fn usb(&self) -> &DEVICE {
        unsafe { (*USB::ptr()).device() }
    }

    fn ep(&self, ep: u8) -> &DEVICE_EP {
        ep_regs(self.usb(), ep & 0b111)
    }

    fn ep_ram(&self, ep: u8) -> &EndpointBank {
        &EP_RAM[(ep & 0b111) as usize][(ep >> 7) as usize]
    }

    fn enable(&mut self) {
        let usb = self.usb();

        usb.ctrla.write(|w| w.swrst().set_bit());
        while usb.syncbusy.read().swrst().bit_is_set() {}

        usb.padcal.write(|w| {
            w.transn().variant(usb_transn_cal());
            w.transp().variant(usb_transp_cal());
            w.trim().variant(usb_trim_cal())
        });

        usb.descadd.write(|w| unsafe { 
            w.descadd().bits(EP_RAM.as_ptr() as u32)
        });

        usb.ctrla.write(|w| {
            w.mode().device();
            w.runstdby().set_bit();
            w.enable().set_bit()
        });

        while usb.syncbusy.read().enable().bit_is_set() {}

        unsafe {
            cortex_m::peripheral::NVIC::unmask(Interrupt::USB);
        }
    }

    pub fn attach(&mut self) {
        self.usb().ctrlb.write(|w| {
            w.spdconf().fs();
            w.detach().clear_bit()
        });
    }

    pub fn detach(&mut self) {
        self.usb().ctrlb.write(|w| {
            w.detach().set_bit()
        });
    }

    async fn bus_event(&self) -> Event {
        self.usb().intenset.write(|w| w.eorst().set_bit());
        NOTIFY_BUS_EVENT.until(|| {
            let flags = self.usb().intflag.read();
            
            if flags.eorst().bit_is_set() {
                self.usb().intflag.write(|w| w.eorst().set_bit());
                Some(Event::Reset)
            } else {
                None
            }
        }).on_cancel(|| {
            self.usb().intenclr.write(|w| w.eorst().set_bit());
        }).await
    }

    fn configure_ep0(&self) {
        let ptr = unsafe { CONTROL_BUF.0.as_mut_ptr() };
        self.ep_ram(0).prepare_out(PacketSize::Size64, ptr, 64);
        self.ep(0).epcfg.write(|w| {
            w.eptype0().variant(1);
            w.eptype1().variant(1)
        })
    }

    async fn receive_setup(&self) -> [u8; 8] {
        let ep_reg = self.ep(0);
        ep_reg.epintenset.write(|w| w.rxstp().set_bit());
        NOTIFY_EP_OUT[0].until(|| {
            ep_reg.epintflag.read().rxstp().bit_is_set()
        }).await;

        // Reading rxstp true means we have access to the setup buffer
        compiler_fence(Ordering::Acquire);

        let setup = unsafe { CONTROL_BUF.0[..8].try_into().unwrap() };

        // once rxstp is cleared, the hardware may receive another packet
        compiler_fence(Ordering::Release);

        ep_reg.epintflag.write(|w| w.rxstp().set_bit());

        setup
    }

    pub async fn handle(&mut self, h: impl Handler) -> Infallible {
        pin_project!{
            #[project = StatePin]
            enum State<F1, F2> {
                Idle,
                Reset{ #[pin] f: F1 },
                Control{ #[pin] f: F2 },
            }
        }

        impl<F1: Future<Output = ()>, F2: Future<Output = ()>> Future for State<F1, F2> {
            type Output = ();
        
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let done = match self.as_mut().project() {
                    StatePin::Idle => Poll::Pending,
                    StatePin::Reset{f} => f.poll(cx),
                    StatePin::Control{f} => f.poll(cx),
                }.is_ready();

                if done {
                    self.set(State::Idle);
                }

                Poll::Pending
            }
        }

        impl<F1: Future<Output = ()>, F2: Future<Output = ()>> FusedFuture for State<F1, F2> {
            fn is_terminated(&self) -> bool {
                false
            }
        }

        let mut inner = pin!(State::Idle);

        loop {
            select_biased! {
                _ = self.bus_event().fuse() => {
                    self.configure_ep0();
                    inner.set(State::Reset { f: h.handle_reset() });
                }
                setup = self.receive_setup().fuse() => {
                    if let Ok(setup) = Setup::parse(self, setup) {
                        inner.set(State::Control { f: self.handle_control(setup, &h) });
                    } else {
                        inner.set(State::Idle);
                        self.stall_ep0();
                    }
                },
                _ = inner => {}
            }
        }
    }

    async fn handle_control<'a>(&self, req: Setup<'a>, h: &impl Handler) {
        use ControlType::*;
        use Recipient::*;
        use ControlData::*;
        use usb::standard_request::{GET_DESCRIPTOR, SET_ADDRESS, SET_CONFIGURATION, SET_INTERFACE, GET_STATUS};
        use usb::descriptor_type;
        debug!("control request: {:?} {:?} {:02x} {:04x} {:04x} {:?}", req.ty, req.recipient, req.request, req.value, req.index, &req.data);
        match req {
            Setup { ty: Standard, request: GET_STATUS, data: In(data), .. } => {
                data.respond(&[0, 0]).await;
            }
            Setup { ty: Standard, recipient: Device, request: GET_DESCRIPTOR, value, index, data: In(data) } => {
                let lang = index;
                let kind = (value >> 8) as u8;
                let index = (value & 0xFF) as u8;
                if kind == descriptor_type::STRING {
                    if let Some(s) = h.get_string_descriptor(index, lang) {
                        data.respond(s.bytes()).await;
                    } else {
                        data.reject();
                        return;
                    }
                } else {
                    if let Some(descriptor) = h.get_descriptor(kind, index) {
                        debug!("returning descriptor");
                        data.respond(descriptor).await;
                    } else {
                        debug!("descriptor not found");
                        data.reject();
                    }
                }
            }
            Setup { ty: Standard, recipient: Device, request: SET_ADDRESS, value, data: Out(data), .. } => {
                debug!("set address {}", value);
                data.accept_and_set_address(value as u8).await;
            }
            Setup { ty: Standard, recipient: Device, request: SET_CONFIGURATION, value, data: Out(data), .. } => {
                debug!("set configuration {}", value);
                match h.set_configuration(value as u8, self).await {
                    Ok(_) => data.accept().await,
                    Err(_) => data.reject(),
                }
            }
            Setup { ty: Standard, recipient: Interface, request: SET_INTERFACE, index, value, data: Out(data), .. } => {
                debug!("set interface {} {}", index, value);
                match h.set_interface(index as u8, value as u8, self).await {
                    Ok(_) => data.accept().await,
                    Err(_) => data.reject(),
                }
            }
            other => h.handle_control(other, self).await
        }
    }

    fn stall_ep0(&self) {
        self.ep(0).epstatusset.write(|w| {
            w.stallrq0().set_bit();
            w.stallrq1().set_bit()
        })
    }

    pub fn configure_ep_in<const EP: u8>(&self) -> Endpoint<In, EP> {
        self.ep(EP).epcfg.modify(|_, w| {
            w.eptype1().variant(3)
        });
        Endpoint { _d: PhantomData } 
    }

    pub fn configure_ep_out<const EP: u8>(&self) -> Endpoint<Out, EP> {
        self.ep(EP).epcfg.modify(|_, w| {
            w.eptype0().variant(3)
        });
        Endpoint { _d: PhantomData } 
    }
}

pub async fn transfer_in(ep: u8, ep_reg: &DEVICE_EP, ep_ram: &EndpointBank, packet_size: PacketSize, ptr: *const u8, len: usize, zlp: bool) {
    ep_ram.prepare_in(packet_size, ptr.cast_mut(), len, zlp);

    // Writing to start the transfer gives hardware control of the buffer
    compiler_fence(Ordering::Release);

    ep_reg.epintflag.write(|w| {
        w.trcpt1().set_bit()
    });
    ep_reg.epstatusset.write(|w| {
        w.bk1rdy().set_bit()
    });
    ep_reg.epintenset.write(|w| {
        w.trcpt1().set_bit()
    });

    NOTIFY_EP_IN[(ep & 0b111) as usize].until(|| {
        ep_reg.epintflag.read().trcpt1().bit_is_set()
    }).on_cancel(|| {
        ep_reg.epstatusclr.write(|w| {
            w.bk1rdy().set_bit()
        });
        ep_reg.epintenclr.write(|w| {
            w.trcpt0().set_bit()
        });
    }).await;

    // Reading trcpt1 means the hardware is done reading the buffer
    compiler_fence(Ordering::Acquire)
}

pub async fn transfer_out(ep: u8, ep_reg: &DEVICE_EP, ep_ram: &EndpointBank, packet_size: PacketSize, ptr: *mut u8, len: usize) -> usize {
    debug_assert!(ep & 0x80 == 0x00);

    ep_ram.prepare_out(packet_size, ptr, len);

    // Writing to start the transfer gives hardware control of the buffer
    compiler_fence(Ordering::Release);

    ep_reg.epintflag.write(|w| {
        w.trcpt0().set_bit();
        w.trfail0().set_bit()
    });
    ep_reg.epstatusclr.write(|w| {
        w.bk0rdy().set_bit()
    });
    ep_reg.epintenset.write(|w| {
        w.trcpt0().set_bit()
    });

    NOTIFY_EP_OUT[(ep & 0b111) as usize].until(|| {
        ep_reg.epintflag.read().trcpt0().bit_is_set()
    }).on_cancel(|| {
        ep_reg.epstatusset.write(|w| {
            w.bk0rdy().set_bit()
        });
        ep_reg.epintenclr.write(|w| {
            w.trcpt0().set_bit()
        });
    }).await;

    // Reading trcpt0 means the hardware is done reading the buffer
    compiler_fence(Ordering::Acquire);

    ep_ram.out_len()
}

static NOTIFY_BUS_EVENT: Notify = Notify::new();

/// Needs to be a `const` to use in array constructor
const NOTIFY: Notify = Notify::new();
static NOTIFY_EP_IN: [Notify; 8] = [NOTIFY; 8];
static NOTIFY_EP_OUT: [Notify; 8] = [NOTIFY; 8];

pub struct In;
pub struct Out;

pub struct Endpoint<D, const EP: u8> {
    _d: PhantomData<D>,
}

impl<D, const EP: u8> Endpoint<D, EP> {
    fn usb(&self) -> &DEVICE {
        unsafe { (*USB::ptr()).device() }
    }

    fn ep(&self) -> &DEVICE_EP {
        ep_regs(self.usb(), EP & 0b111)
    }
}

impl<const EP: u8> Endpoint<Out, EP> {
    fn ep_ram(&self) -> &EndpointBank {
        &EP_RAM[(EP & 0b111) as usize][0]
    }

    pub async fn transfer<const SIZE: usize>(&self, buf: &mut UsbBuffer<SIZE>) -> usize {
        transfer_out(EP, self.ep(), self.ep_ram(), PacketSize::Size64, buf.0.as_mut_ptr(), buf.0.len()).await
    }
}

impl<const EP: u8> Endpoint<In, EP> {
    fn ep_ram(&self) -> &EndpointBank {
        &EP_RAM[(EP & 0b111) as usize][1]
    }

    pub async fn transfer<const SIZE: usize>(&self, buf: &UsbBuffer<SIZE>, len: usize, zlp: bool) {
        assert!(len < SIZE);
        transfer_in(EP, self.ep(), self.ep_ram(), PacketSize::Size64, buf.0.as_ptr(), len, zlp).await
    }
}

#[interrupt]
fn USB() {
    let usb = unsafe { USB::steal() };
    let usb = usb.device();

    let flags = usb.intflag.read();
    if flags.eorst().bit_is_set() {
        usb.intenclr.write(|w| w.eorst().set_bit());
        NOTIFY_BUS_EVENT.notify();
    }

    let summary = usb.epintsmry.read().bits();

    for ep in 0..8 {
        let mask = 1 << ep;
        if summary & mask != 0 {
            let regs = ep_regs(usb, ep);
            let flags = regs.epintflag.read();
            let enables = regs.epintenclr.read();

            if flags.rxstp().bit() & enables.rxstp().bit() {
                regs.epintenclr.write(|w| w.rxstp().set_bit());
                NOTIFY_EP_OUT[ep as usize].notify();
            }
            
            if flags.trcpt0().bit() & enables.trcpt0().bit() {
                regs.epintenclr.write(|w| w.trcpt0().set_bit());
                NOTIFY_EP_OUT[ep as usize].notify();
            }

            if flags.trcpt1().bit() & enables.trcpt1().bit() {
                regs.epintenclr.write(|w| w.trcpt1().set_bit());
                NOTIFY_EP_IN[ep as usize].notify();
            }
        }
    }
}