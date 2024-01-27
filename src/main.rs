#![no_std]
#![no_main]

use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::image::{ImageRaw, Image};
use embedded_graphics::pixelcolor::BinaryColor;
use panic_halt as _;

pub use atsamd_hal as hal;

pub use cortex_m_rt::entry;

pub use hal::pac;

use hal::clock::GenericClockController;
use hal::pac::{CorePeripherals, Peripherals};
use hal::{prelude::*, gpio, sercom::i2c };
use ssd1306::mode::DisplayConfig;
use ssd1306::rotation::DisplayRotation;
use ssd1306::size::DisplaySize128x64;
use ssd1306::{I2CDisplayInterface, Ssd1306};

use rtt_target::{rprintln, debug_rtt_init_print};

mod pin;
mod onewire;
mod delay;

#[entry]
fn main() -> ! {
    let mut peripherals = unsafe { Peripherals::steal() };
    let core = unsafe { CorePeripherals::steal() };
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let pins = gpio::Pins::new(peripherals.PORT);

    debug_rtt_init_print!();

    let gclk0 = clocks.gclk0();
    let sercom0_clock = &clocks.sercom0_core(&gclk0).unwrap();
    let i2c = i2c::Config::new(&peripherals.PM, peripherals.SERCOM0, i2c::Pads::new(pins.pa08, pins.pa09), sercom0_clock.freq())
        .baud(100.kHz())
        .enable();

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("../rust.raw"), 64);

    let im = Image::new(&raw, Point::new(32, 0));

    im.draw(&mut display).unwrap();

    display.flush().unwrap();

    let mut delay = delay::Delay::<48_000_000>::new(core.SYST);

    let mut w1 = onewire::Onewire::new(delay.clone(), pins.pa04);

    loop {
        let value = onewire::ds18b20_read(&mut w1);
        rprintln!("{}", value * 100 / 16);
        delay.delay_ms(200u32);
    }
}