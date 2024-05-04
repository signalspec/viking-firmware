#![no_std]
#![no_main]
#![allow(unused)] // TODO

use core::borrow::BorrowMut;
use core::cell::{Cell, RefCell, UnsafeCell};
use core::convert::Infallible;
use core::future::{Future, poll_fn};
use core::marker::PhantomData;
use core::ops::Not;
use core::pin::pin;
use core::ptr::addr_of_mut;
use core::task::Poll;

use hal::gpio::{Alternate, Pin, C};
use lilos::exec::Notify;
use panic_probe as _;
use defmt_rtt as _;

pub use atsamd_hal as hal;

pub use cortex_m_rt::entry;

pub use hal::pac;

use hal::clock::GenericClockController;
use hal::pac::{CorePeripherals, Peripherals};
use hal::{gpio, prelude::*, serial_number };

use defmt::{error, info};

mod usb;
mod viking;
mod viking_sam0;
mod delay;

static mut BULK_OUT_BUF: UsbBuffer<128> = UsbBuffer::new();
static mut BULK_IN_BUF: UsbBuffer<128> = UsbBuffer::new();

#[entry]
fn main() -> ! {
    let mut peripherals = unsafe { Peripherals::steal() };
    let mut core = unsafe { CorePeripherals::steal() };
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let pins = gpio::Pins::new(peripherals.PORT);

    info!("init");
    let mut delay = delay::Delay::<48_000_000>::new(core.SYST);

    let gclk0 = clocks.gclk0();

    peripherals.PM.apbcmask.write(|w| {
        w.sercom1_().set_bit()
    });

    peripherals.PM.ahbmask.write(|w| {
        w.usb_().set_bit()
    });

    let usb_clock = &clocks.usb(&gclk0).unwrap();

    let mut usb = usb::Usb::new(&usb_clock, pins.pa24, pins.pa25, peripherals.USB);
    usb.attach();

    let bulk_eps = Cell::new(None);
    let bulk_eps_notify = Notify::new();

    let viking = RefCell::new(viking_impl::State::new());

    struct Handler<'a> {
        bulk_eps: &'a Cell<Option<(usb::Endpoint<Out, 0x01>, usb::Endpoint<In, 0x82>)>>,
        bulk_eps_notify: &'a Notify,
        viking: &'a RefCell<viking_impl::State>,
    }
    impl usb::Handler for Handler<'_> {
        fn get_descriptor(&self, kind: u8, index: u8) -> Option<&[u8]> {
            use ::usb::descriptor_type::{CONFIGURATION, DEVICE, BOS};
            match (kind, index) {
                (DEVICE, _) => Some(DEVICE_DESCRIPTOR),
                (CONFIGURATION, 0) => Some(CONFIG_DESCRIPTOR),
                (BOS, 0) => Some(BOS_DESCRIPTOR),
                _ => None,
            }
        }

        fn get_string_descriptor(&self, index: u8, _lang: u16) -> Option<StringDecriptor<128>> {
            match index {
                0 => Some(StringDecriptor::language_list()),
                STRING_MFG => Some(StringDecriptor::new("signalspec project")),
                STRING_PRODUCT => Some(StringDecriptor::new("samd21 test device")),
                STRING_SERIAL => Some(StringDecriptor::new_hex(&serial_number())),
                _ => None,
            }
        }

        async fn set_configuration(&self, cfg: u8, usb: &Usb) -> Result<(), ()> {
            if cfg == 1 {
                let ep_out = usb.configure_ep_out::<0x01>();
                let ep_in = usb.configure_ep_in::<0x82>();

                self.bulk_eps.set(Some((ep_out, ep_in)));
                self.bulk_eps_notify.notify();

                Ok(())
            } else {
                Err(())
            }
        }

        async fn set_interface(&self, intf: u8, alt: u8, usb: &Usb) -> Result<(), ()> {
            if intf == 0 && alt == 0 {
                Ok(())
            } else {
                Err(())
            }
        }

        async fn handle_control<'a>(&self, req: Setup<'a>, usb: &Usb) {
            use usb::ControlType::*;
            use usb::Recipient::*;
            use usb::ControlData::{In, Out};

            pub const I_VIKING: u16 = INTF_VIKING as u16;

            use viking_protocol::request::{CONFIGURE_MODE, DESCRIBE_MODE, LIST_MODES, LIST_RESOURCES};

            match req {
                Setup { ty: Vendor, recipient: Device, request: MSOS_VENDOR_CODE, index: 0x07, data: In(data), .. } => {
                    data.respond(&MSOS_DESCRIPTOR).await;
                }

                Setup { ty: Vendor, recipient: Interface, index: I_VIKING, request: LIST_RESOURCES, data: In(data), .. } => {
                    data.respond(viking_impl::State::RESOURCE_NAMES.as_bytes()).await;
                }

                Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: LIST_MODES, data: In(data), .. } => {
                    let resource = (value >> 8) as u8;
                    if let Some(modes) = viking_impl::State::mode_names(resource) {
                        data.respond(modes.as_bytes()).await;
                    } else {
                        data.reject();
                    }
                }

                Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: DESCRIBE_MODE, data: In(data), .. } => {
                    let resource = (value >> 8) as u8;
                    let mode = (value & 0xff) as u8;
                    if let Some(mode) = viking_impl::State::describe(resource, mode) {
                        data.respond(mode).await;
                    } else {
                        data.reject();
                    }
                }

                Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: CONFIGURE_MODE, data: Out(data), .. } => {
                    let resource = (value >> 8) as u8;
                    let mode = (value & 0xff) as u8;
                    info!("configure {} {}", resource, mode);

                    let Ok(mut resources) = self.viking.try_borrow_mut() else {
                        error!("busy");
                        data.reject();
                        return;
                    };

                    if resources.configure(resource, mode, &[]).await.is_ok() {
                        data.accept().await;
                    } else {
                        error!("configure failed");
                        data.reject();
                    }
                }

                unknown => unknown.reject(),
            }
        }
    }

    let usb_task = pin!(usb.handle(Handler {
        bulk_eps: &bulk_eps,
        bulk_eps_notify: &bulk_eps_notify,
        viking: &viking
    }));

    let bulk_task = pin!(async {
        loop {
            let buf_out = unsafe { &mut *addr_of_mut!(BULK_OUT_BUF) };
            let buf_in = unsafe { &mut *addr_of_mut!(BULK_IN_BUF) };
            let (ep_out, ep_in) = bulk_eps_notify.until(|| bulk_eps.take() ).await;

            loop {
                let len = ep_out.transfer(buf_out).await;
                info!("bulk read {}: {:?}", len, &buf_out[..len]);

                let mut response = Writer::new(&mut buf_in[..], 1);
                
                let status = match viking.borrow().run(&buf_out[..len], &mut response, &mut delay).await {
                    Ok(_) => 0,
                    Err(_) => 1,
                };
                
                let response_len = response.offset();
                buf_in[0] = status;
                ep_in.transfer(buf_in, response_len, true).await;
                info!("bulk write complete");
            }
        }
    });

    lilos::exec::run_tasks(
        &mut [/*led_task,*/ usb_task, bulk_task],
        lilos::exec::ALL_TASKS,
    );
}

use usb::descriptors::{Config, Device, Endpoint, Interface, StringDecriptor, BinaryObjectStore, PlatformCapabilityMicrosoftOs, MicrosoftOs, MicrosoftOsConfiguration, MicrosoftOsFunction, MicrosoftOsCompatibleID, MicrosoftOsDeviceInterfaceGUID};
use usb::{In, UsbBuffer};

use crate::usb::{Out, Setup, Usb};
use crate::viking::{Resources, Writer};

const INTF_VIKING: u8 = 0;

const STRING_MFG: u8 = 1;
const STRING_PRODUCT: u8 = 2;
const STRING_SERIAL: u8 = 3;

static DEVICE_DESCRIPTOR: &[u8] = descriptors! {
    Device {
        bcdUSB: 0x0201,
        bDeviceClass: ::usb::class::VENDOR_SPECIFIC,
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
                bEndpointAddress: 0x01,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }

            +Endpoint {
                bEndpointAddress: 0x82,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }

            +Endpoint {
                bEndpointAddress: 0x83,
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

viking!(
    viking_impl {
        use {
            crate::viking_sam0,
            atsamd_hal::gpio::*,
        };

        pa10(1) {
            gpio(1): viking_sam0::Gpio<PA10>,
        }
        pa11(2) {
            gpio(1): viking_sam0::Gpio<PA11>,
        }
        pb30(3) {
            gpio(1): viking_sam0::Gpio<PB30>,
        }
    }
);
