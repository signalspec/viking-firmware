use zeptos::usb::Usb;

mod gpio;
pub use gpio::{Gpio, LevelInterrupt, Led};

pub struct Platform {

}

impl Platform {
    pub fn new(rt: zeptos::Runtime, hw: zeptos::Hardware) -> (Usb, Platform) {
        gpio::init(rt);
        (hw.usb, Platform {})
    }
}
