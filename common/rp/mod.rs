use zeptos::usb::Usb;

mod gpio;
pub use gpio::{Gpio, Led};

pub struct Platform {

}

impl Platform {
    pub fn new(_rt: zeptos::Runtime, hw: zeptos::Hardware) -> (Usb, Platform) {
        (hw.usb, Platform {})
    }
}
