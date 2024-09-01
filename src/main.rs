#![no_std]
#![no_main]
//#![allow(unused)] // TODO
#![feature(impl_trait_in_assoc_type)]

use core::cell::RefCell;
use core::convert::Infallible;
use core::future::{Future, poll_fn};
use core::mem;
use core::pin::pin;
use core::ptr::addr_of_mut;
use core::task::Poll;

use futures_util::future::{Fuse, FusedFuture};
use futures_util::FutureExt;
use panic_probe as _;
use defmt_rtt as _;

use defmt::info;

mod viking;
mod viking_sam0;
mod delay;

static mut BULK_OUT_BUF: UsbBuffer<128> = UsbBuffer::new();
static mut BULK_IN_BUF: UsbBuffer<128> = UsbBuffer::new();
static mut EVT_IN_BUF1: UsbBuffer<64> = UsbBuffer::new();
static mut EVT_IN_BUF2: UsbBuffer<64> = UsbBuffer::new();

#[zeptos::main]
async fn main(rt: zeptos::Runtime, mut hw: zeptos::Hardware) {
    info!("init");

    let pm = unsafe { zeptos::samd::pac::PM::steal() };
    let mut gclk = unsafe { zeptos::samd::pac::GCLK::steal() };
    let eic = unsafe { zeptos::samd::pac::EIC::steal() };

    pm.apbcmask.write(|w| {
        w.sercom0_().set_bit();
        w.sercom1_().set_bit()
    });

    eic.ctrl.write(|w| w.enable().set_bit());

    zeptos::samd::clock::enable_clock(&mut gclk, zeptos::samd::pac::gclk::clkctrl::IDSELECT_A::SERCOM0_CORE, zeptos::samd::pac::gclk::clkctrl::GENSELECT_A::GCLK0);
    zeptos::samd::clock::enable_clock(&mut gclk, zeptos::samd::pac::gclk::clkctrl::IDSELECT_A::SERCOM1_CORE, zeptos::samd::pac::gclk::clkctrl::GENSELECT_A::GCLK0);

    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::SERCOM0);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EIC);
    }

    let viking = RefCell::new(viking_impl::State::new());
    let syst = RefCell::new(hw.syst);

    hw.usb.run_device(Handler {
        rt,
        syst: unsafe { mem::transmute(&syst) },
        viking: unsafe { mem::transmute(&viking) },
    }).await;
}

struct Handler {
    rt: Runtime,
    syst: &'static RefCell<SysTick>,
    viking: &'static RefCell<viking_impl::State>,
}

impl zeptos::usb::Handler for Handler {
    fn get_descriptor<'a>(&self, kind: u8, index: u8, _lang: u16, builder: &'a mut DescriptorBuilder) -> Option<&'a [u8]> {
        use ::usb::descriptor_type::{CONFIGURATION, DEVICE, BOS, STRING};
        match (kind, index) {
            (DEVICE, _) => Some(DEVICE_DESCRIPTOR),
            (CONFIGURATION, 0) => Some(CONFIG_DESCRIPTOR),
            (BOS, 0) => Some(BOS_DESCRIPTOR),
            (STRING, 0) => Some(LANGUAGE_LIST_US_ENGLISH),
            (STRING, STRING_MFG) => Some(builder.string_ascii("signalspec project")),
            (STRING, STRING_PRODUCT) => Some(builder.string_ascii("samd21 test device")),
            (STRING, STRING_SERIAL) => Some(builder.string_hex(&zeptos::samd::serial_number())),
            _ => None,
        }
    }

    async fn set_configuration(&self, cfg: u8, usb: &mut Endpoints) -> Result<(), ()> {
        if cfg == 1 {
            bulk_task(self.rt).cancel();
            evt_task(self.rt).cancel();

            let ep_out = usb.bulk_out::<0x01>();
            let ep_in = usb.bulk_in::<0x82>();
            let ep_evt = usb.bulk_in::<0x83>();

            bulk_task(self.rt).spawn(self.viking, ep_out, ep_in, self.syst);
            evt_task(self.rt).spawn(self.viking, ep_evt);
            Ok(())
        } else {
            Err(())
        }
    }

    async fn set_interface(&self, intf: u8, alt: u8, _usb: &mut Endpoints) -> Result<(), ()> {
        if intf == 0 && alt == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    async fn handle_control<'a>(&self, req: Setup<'a>) -> Responded {
        use zeptos::usb::ControlType::*;
        use zeptos::usb::Recipient::*;
        use zeptos::usb::ControlData::{In, Out};

        pub const I_VIKING: u16 = INTF_VIKING as u16;

        use viking_protocol::request::{CONFIGURE_MODE, DESCRIBE_MODE, LIST_MODES, LIST_RESOURCES};

        match req {
            Setup { ty: Vendor, recipient: Device, request: MSOS_VENDOR_CODE, index: 0x07, data: In(data), .. } => {
                data.respond(&MSOS_DESCRIPTOR).await
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, request: LIST_RESOURCES, data: In(data), .. } => {
                data.respond(viking_impl::State::RESOURCE_NAMES.as_bytes()).await
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: LIST_MODES, data: In(data), .. } => {
                let resource = (value >> 8) as u8;
                if let Some(modes) = viking_impl::State::mode_names(resource) {
                    data.respond(modes.as_bytes()).await
                } else {
                    data.reject()
                }
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: DESCRIBE_MODE, data: In(data), .. } => {
                let resource = (value >> 8) as u8;
                let mode = (value & 0xff) as u8;
                if let Some(mode) = viking_impl::State::describe(resource, mode) {
                    data.respond(mode).await
                } else {
                    data.reject()
                }
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: CONFIGURE_MODE, data: Out(data), .. } => {
                let resource = (value >> 8) as u8;
                let mode = (value & 0xff) as u8;
                info!("configure {} {}", resource, mode);

                let Ok(mut resources) = self.viking.try_borrow_mut() else {
                    info!("busy");
                    return data.reject();
                };

                if resources.configure(resource, mode, &[]).is_ok() {
                    data.accept().await
                } else {
                    data.reject()
                }
            }

            unknown => unknown.reject(),
        }
    }
}

#[zeptos::task]
async fn bulk_task(viking: &'static RefCell<viking_impl::State>, mut ep_out: zeptos::usb::Endpoint<Out, EP_OUT>, mut ep_in: zeptos::usb::Endpoint<In, EP_IN>, delay: &'static RefCell<SysTick>) {
    loop {
        let buf_out = unsafe { &mut *addr_of_mut!(BULK_OUT_BUF) };
        let buf_in = unsafe { &mut *addr_of_mut!(BULK_IN_BUF) };

        loop {
            let len = ep_out.receive(buf_out).await;
            info!("bulk read {}: {:?}", len, &buf_out[..len]);

            let mut response = Writer::new(&mut buf_in[..], 1);
            
            let status = match viking.borrow().run(&buf_out[..len], &mut response, &mut *delay.borrow_mut()).await {
                Ok(_) => 0,
                Err(_) => 1,
            };
            
            let response_len = response.offset();
            buf_in[0] = status;
            ep_in.send(buf_in, response_len, true).await;
            info!("bulk write complete");
        }
    }
}

#[zeptos::task]
async fn evt_task(viking: &'static RefCell<viking_impl::State>, mut ep_evt: zeptos::usb::Endpoint<In, EP_EVT>,) {
    let ep_evt = RefCell::new(ep_evt);
    loop {
        let mut buf_fill = unsafe { addr_of_mut!(EVT_IN_BUF1) };
        let mut buf_send = unsafe { addr_of_mut!(EVT_IN_BUF2) };

        let mut transfer = pin!(Fuse::terminated());
        let mut buf = Writer::new(unsafe { &mut (*buf_fill)[..] }, 0);

        poll_fn(|cx| -> Poll<Infallible> {
            //EVENT_CHANGE.subscribe(cx.waker());
            viking.borrow().poll_all(cx.waker(), &mut buf);

            let _ = transfer.as_mut().poll(cx);
            info!("Events: {:?} {} {:?}", buf_fill, buf.offset(), transfer.is_terminated());

            if transfer.is_terminated() && buf.offset() > 0 {
                let len = buf.offset();
                buf = Writer::new(unsafe { &mut (*buf_send)[..] }, 0);
                info!("Sending events: {}", len);
                let mut ep_evt = ep_evt.borrow_mut();
                transfer.set(async move { ep_evt.send(unsafe { &mut *buf_fill }, len, true).await }.fuse());
                let _ = transfer.as_mut().poll(cx);
                mem::swap(&mut buf_fill, &mut buf_send);
            }

            Poll::Pending
        }).await;
    }
}

use zeptos::cortex_m::SysTick;
use zeptos::samd::pac::Interrupt;
use zeptos::usb::descriptors::{descriptors, BinaryObjectStore, Config, DescriptorBuilder, Device, Endpoint, Interface, MicrosoftOs, MicrosoftOsCompatibleID, PlatformCapabilityMicrosoftOs, LANGUAGE_LIST_US_ENGLISH };
use zeptos::usb::{Endpoints, In, Out, Responded, Setup, UsbBuffer};
use zeptos::Runtime;

use crate::viking::{Resources, Writer};

const INTF_VIKING: u8 = 0;

const STRING_MFG: u8 = 1;
const STRING_PRODUCT: u8 = 2;
const STRING_SERIAL: u8 = 3;

const EP_OUT: u8 = 0x01;
const EP_IN: u8 = 0x82;
const EP_EVT: u8 = 0x83;

static DEVICE_DESCRIPTOR: &[u8] = descriptors! {
    Device {
        bcdUSB: 0x0201,
        bDeviceClass: usb::class_code::VENDOR_SPECIFIC,
        bDeviceSubClass: 0x00,
        bDeviceProtocol: 0x00,
        bMaxPacketSize0: 64,
        idVendor: 0x59e3,
        idProduct: 0x2222,
        bcdDevice: 0x0000,
        iManufacturer: STRING_MFG,
        iProduct: STRING_PRODUCT,
        iSerialNumber: STRING_SERIAL,
        bNumConfigurations: 1,
    }
};

static CONFIG_DESCRIPTOR: &[u8] = descriptors!{
    Config {
        bConfigurationValue: 1,
        iConfiguration: 0,
        bmAttributes: 0x80,
        bMaxPower: 250,

        +Interface {
            bInterfaceNumber: INTF_VIKING,
            bAlternateSetting: 0,
            bInterfaceClass: 0xff,
            bInterfaceSubClass: 0,
            bInterfaceProtocol: 0,
            iInterface: 0,

            +Endpoint {
                bEndpointAddress: EP_OUT,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }

            +Endpoint {
                bEndpointAddress: EP_IN,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }

            +Endpoint {
                bEndpointAddress: EP_EVT,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }
        }
    }
};

const MSOS_DESCRIPTOR: &[u8] = descriptors!{
    MicrosoftOs {
        windows_version: 0x06030000,

        +MicrosoftOsCompatibleID {
            compatible_id: "WINUSB",
            sub_compatible_id: "",
        }
    }
};

const MSOS_VENDOR_CODE: u8 = 0xf0;

static BOS_DESCRIPTOR: &[u8] = descriptors!{
    BinaryObjectStore {
        +PlatformCapabilityMicrosoftOs {
            windows_version: 0x06030000,
            vendor_code: MSOS_VENDOR_CODE,
            alt_enum_code: 0,
            msos_descriptor_len: MSOS_DESCRIPTOR.len(),
        }
    }
};

viking::viking!(
    viking_impl {
        use {
            zeptos::samd::gpio::*,
            zeptos::samd::gpio::alternate::*,
            crate::viking_sam0,
            crate::viking_sam0::Sercom0,
        };

        pa08(1) {
            gpio(1): viking_sam0::Gpio<PA08>,
            sercom0_i2c_sda(2): viking_sam0::SercomSCLPin<PA08, Sercom0, C>,
            sercom0_spi_so(3): viking_sam0::SercomSOPin<PA08, Sercom0, C>,
        }
        pa09(2) {
            gpio(1): viking_sam0::Gpio<PA09>,
            sercom0_i2c_scl(2): viking_sam0::SercomSDAPin<PA09, Sercom0, C>,
            sercom0_spi_sck(3): viking_sam0::SercomSCKPin<PA09, Sercom0, C>,
        }
        pa10(3) {
            gpio(1): viking_sam0::Gpio<PA10>,
            sercom0_spi_si(2): viking_sam0::SercomSIPin<PA10, Sercom0, C>,
        }
        pa11(4) {
            gpio(1): viking_sam0::Gpio<PA11>,
            level_interrupt(2): viking_sam0::LevelInterrupt<PA11, 11>,
        }
        pb30(5) {
            gpio(1): viking_sam0::Gpio<PB30>,
        }
        sercom0(6) {
            i2c(1): viking_sam0::SercomI2C<Sercom0>,
            spi(2): viking_sam0::SercomSPI<Sercom0, 0, 2>,
        }
    }
);
