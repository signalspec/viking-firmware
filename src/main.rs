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
use lilos::time::{sleep_for, Millis};
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

    let gclk0 = clocks.gclk0();

    peripherals.PM.apbcmask.write(|w| {
        w.sercom1_().set_bit()
    });

    peripherals.PM.ahbmask.write(|w| {
        w.usb_().set_bit()
    });

    //let mut led = pins.pb30.into_push_pull_output();

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
            use ::usb::descriptor_type::{CONFIGURATION, DEVICE};
            match (kind, index) {
                (DEVICE, _) => Some(DEVICE_DESCRIPTOR),
                (CONFIGURATION, 0) => Some(CONFIG_DESCRIPTOR),
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

        async fn handle_control<'a>(&self, req: Setup<'a>, usb: &Usb) {
            use usb::ControlType::*;
            use usb::Recipient::*;
            use usb::ControlData::{In, Out};

            pub const I_VIKING: u16 = INTF_VIKING as u16;

            use viking_protocol::request::{CONFIGURE_MODE, DESCRIBE_MODE, LIST_MODES, LIST_RESOURCES};

            match req {
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

    /*let led_task = pin!(async {
        loop {
            led.set_high().unwrap();
            sleep_for(Millis(1000)).await;
            led.set_low().unwrap();
            sleep_for(Millis(1000)).await;
            info!("blink");
        }
    });*/

    let bulk_task = pin!(async {
        loop {
            let buf_out = unsafe { &mut *addr_of_mut!(BULK_OUT_BUF) };
            let buf_in = unsafe { &mut *addr_of_mut!(BULK_IN_BUF) };
            let (ep_out, ep_in) = bulk_eps_notify.until(|| bulk_eps.take() ).await;

            loop {
                let len = ep_out.transfer(buf_out).await;
                info!("bulk read {}: {:?}", len, &buf_out[..len]);
                
                let status = match viking.borrow().run(&buf_out[..len]).await {
                    Ok(_) => 0,
                    Err(_) => 1,
                };

                buf_in[1] = status;
                ep_in.transfer(buf_in, 1, true).await;
                info!("bulk write complete");
            }
        }
    });

    lilos::time::initialize_sys_tick(
        &mut core.SYST,
        48_000_000,
    );
    lilos::exec::run_tasks(
        &mut [/*led_task,*/ usb_task, bulk_task],
        lilos::exec::ALL_TASKS,
    );
}

use usb::descriptors::{Config, Device, Endpoint, Interface, StringDecriptor};
use usb::{In, UsbBuffer};

use crate::usb::{Out, Setup, Usb};
use crate::viking::Resources;

const INTF_VIKING: u8 = 0;

const STRING_MFG: u8 = 1;
const STRING_PRODUCT: u8 = 2;
const STRING_SERIAL: u8 = 3;

static DEVICE_DESCRIPTOR: &[u8] = descriptors! {
    Device {
        bcdUSB: 0x0200,
        bDeviceClass: 0xFF,
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
