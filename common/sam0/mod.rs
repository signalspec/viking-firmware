use zeptos::usb::Usb;
use zeptos::cortex_m::SysTick;

mod sercom;
pub use sercom::{ Sercom0, Sercom1, Sercom2 };

mod gpio;
pub use gpio::{Gpio, LevelInterrupt, Led};

mod i2c;
pub use i2c::{SercomI2C, SercomSCLPin, SercomSDAPin};

mod spi;
pub use spi::{SercomSPI, SercomSCKPin, SercomSDOPin, SercomSDIPin};

pub struct Platform {

}

impl Platform {
    pub fn new(_rt: zeptos::Runtime, hw: zeptos::Hardware) -> (Usb, SysTick, Platform) {
        (hw.usb, hw.syst, Platform {})
    }
}

impl crate::Platform for Platform {

}