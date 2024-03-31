#![no_std]
#![no_main]
#![allow(unused)] // TODO

use core::pin::pin;

use hal::gpio::{Alternate, Pin, C};
use lilos::time::{sleep_for, Millis};
use panic_probe as _;
use defmt_rtt as _;

pub use atsamd_hal as hal;

pub use cortex_m_rt::entry;

pub use hal::pac;

use hal::clock::GenericClockController;
use hal::pac::{CorePeripherals, Peripherals};
use hal::{gpio, prelude::*, serial_number };

use defmt::info;

mod usb;

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

    let mut led = pins.pb30.into_push_pull_output();

    let usb_clock = &clocks.usb(&gclk0).unwrap();

    let mut usb = usb::Usb::new(&usb_clock, pins.pa24, pins.pa25, peripherals.USB);
    usb.attach();

    struct Handler;
    impl usb::Handler for Handler {
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
    }

    let usb_task = pin!(usb.handle(Handler));

    let led_task = pin!(async {
        loop {
            led.set_high().unwrap();
            sleep_for(Millis(1000)).await;
            led.set_low().unwrap();
            sleep_for(Millis(1000)).await;
            info!("blink");
        }
    });

    lilos::time::initialize_sys_tick(
        &mut core.SYST,
        48_000_000,
    );
    lilos::exec::run_tasks(
        &mut [led_task, usb_task],
        lilos::exec::ALL_TASKS,
    );
}

use usb::descriptors::{Config, Device, Endpoint, Interface, StringDecriptor};

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
            bInterfaceNumber: 0,
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
