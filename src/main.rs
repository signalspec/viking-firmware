#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(macro_metavar_expr)]
#![feature(inline_const_pat)]

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
use zeptos::cortex_m::SysTick;
use zeptos::usb::descriptors::{descriptors, BinaryObjectStore, Config, DescriptorBuilder, Device, Endpoint, Interface, MicrosoftOs, MicrosoftOsCompatibleID, PlatformCapabilityMicrosoftOs, LANGUAGE_LIST_US_ENGLISH };
use zeptos::usb::{Endpoints, In, Out, Responded, Setup, UsbBuffer};
use zeptos::Runtime;
use viking::{Resources, Writer};

mod viking;
mod delay;

mod board;

static mut BULK_OUT_BUF: UsbBuffer<128> = UsbBuffer::new();
static mut BULK_IN_BUF: UsbBuffer<128> = UsbBuffer::new();
static mut EVT_IN_BUF1: UsbBuffer<64> = UsbBuffer::new();
static mut EVT_IN_BUF2: UsbBuffer<64> = UsbBuffer::new();

#[zeptos::main]
async fn main(rt: zeptos::Runtime, mut hw: zeptos::Hardware) {
    info!("init");

    board::init();

    let viking = RefCell::new(board::viking_impl::State::new());
    let syst = RefCell::new(hw.syst);

    hw.usb.run_device(&mut Handler {
        rt,
        syst: unsafe { mem::transmute(&syst) },
        viking: unsafe { mem::transmute(&viking) },
    }).await;
}

struct Handler {
    rt: Runtime,
    syst: &'static RefCell<SysTick>,
    viking: &'static RefCell<board::viking_impl::State>,
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
            (STRING, STRING_PRODUCT) => Some(builder.string_ascii(board::PRODUCT_STRING)),
            (STRING, STRING_SERIAL) => Some(builder.string_hex(&board::serial_number())),
            _ => None,
        }
    }

    async fn set_configuration(&self, cfg: u8, usb: &mut Endpoints) -> Result<(), ()> {
        if cfg == 1 {
            self.set_interface(0, 0, usb).await
        } else {
            Err(())
        }
    }

    async fn set_interface(&self, intf: u8, alt: u8, usb: &mut Endpoints) -> Result<(), ()> {
        if intf == 0 {
            bulk_task(self.rt).cancel();
            evt_task(self.rt).cancel();

            self.viking.borrow_mut().reset_all();

            if alt == 1 {
                info!("Enabling interface");
                let ep_out = usb.bulk_out::<EP_OUT>();
                let ep_in = usb.bulk_in::<EP_IN>();
                let ep_evt = usb.bulk_in::<EP_EVT>();
    
                bulk_task(self.rt).spawn(self.viking, ep_out, ep_in, self.syst);
                evt_task(self.rt).spawn(self.viking, ep_evt);
            } else {
                info!("Disabled interface");
            }

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

        use viking_protocol::request::{DESCRIBE_RESOURCES, CONFIGURE_MODE};

        match req {
            Setup { ty: Vendor, recipient: Device, request: MSOS_VENDOR_CODE, index: 0x07, data: In(data), .. } => {
                data.respond(&MSOS_DESCRIPTOR).await
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, request: DESCRIBE_RESOURCES, data: In(data), .. } => {
                data.respond(board::viking_impl::State::DESCRIPTOR).await
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: CONFIGURE_MODE, data: Out(data), .. } => {
                let resource = (value >> 8) as u8;
                let mode = (value & 0xff) as u8;
                info!("configure {} {}", resource, mode);

                let Ok(mut resources) = self.viking.try_borrow_mut() else {
                    info!("resource busy");
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
async fn bulk_task(viking: &'static RefCell<board::viking_impl::State>, mut ep_out: zeptos::usb::Endpoint<Out, EP_OUT>, mut ep_in: zeptos::usb::Endpoint<In, EP_IN>, delay: &'static RefCell<SysTick>) {
    loop {
        let buf_out = unsafe { &mut *addr_of_mut!(BULK_OUT_BUF) };
        let buf_in = unsafe { &mut *addr_of_mut!(BULK_IN_BUF) };

        loop {
            let len = ep_out.receive(buf_out).await;
            info!("bulk read {}: {:?}", len, &buf_out[..len]);

            if len < 2 {
                continue;
            }
            let sync = buf_out[0];

            let mut response = Writer::new(&mut buf_in[..], 2);
            
            let status = match viking.borrow().run(&buf_out[2..len], &mut response, &mut *delay.borrow_mut()).await {
                Ok(_) => 0,
                Err(_) => 1,
            };
            
            let response_len = response.offset();
            buf_in[0] = sync;
            buf_in[1] = status;
            ep_in.send(buf_in, response_len, true).await; //todo zlp
            info!("bulk write complete");
        }
    }
}

#[zeptos::task]
async fn evt_task(viking: &'static RefCell<board::viking_impl::State>, ep_evt: zeptos::usb::Endpoint<In, EP_EVT>,) {
    let ep_evt = RefCell::new(ep_evt);
    loop {
        let mut buf_fill = &raw mut EVT_IN_BUF1;
        let mut buf_send = &raw mut EVT_IN_BUF2;

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
        }

        +Interface {
            bInterfaceNumber: INTF_VIKING,
            bAlternateSetting: 1,
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
