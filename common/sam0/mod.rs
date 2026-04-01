use zeptos::usb::Usb;

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
    pub fn new(_rt: zeptos::Runtime, hw: zeptos::Hardware) -> (Usb, Platform) {
        (hw.usb, Platform {})
    }
}
