#![no_std]
#![no_main]

use panic_halt as _;

pub use atsamd_hal as hal;

pub use cortex_m_rt::entry;

pub use hal::pac;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::pac::{CorePeripherals, Peripherals};
use hal::{prelude::*, gpio};


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

    let mut led = pins.pb30.into_push_pull_output();
    let mut delay = Delay::new(core.SYST, &mut clocks);

    loop {
        delay.delay_ms(200u32);
        led.set_high().unwrap();
        delay.delay_ms(200u32);
        led.set_low().unwrap();
    }
}