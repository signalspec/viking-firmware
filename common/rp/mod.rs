use zeptos::usb::Usb;

mod gpio;
pub use gpio::{Gpio, LevelInterrupt, Led};

mod i2c;
pub use i2c::{I2c, I2cSdaPin, I2cSclPin};

mod spi;
pub use spi::{Spi, SpiSckPin, SpiSdoPin, SpiSdiPin};

pub struct Platform {

}

impl Platform {
    pub fn new(rt: zeptos::Runtime, hw: zeptos::Hardware) -> (Usb, Platform) {
        gpio::init(rt);
        (hw.usb, Platform {})
    }
}
