use zeptos::usb::Usb;
use zeptos::cortex_m::SysTick;

mod gpio;
pub use gpio::{Gpio, Led};

pub struct Platform {

}

impl Platform {
    pub fn new(_rt: zeptos::Runtime, hw: zeptos::Hardware) -> (Usb, SysTick, Platform) {
        (hw.usb, hw.syst, Platform {})
    }
}

impl crate::Platform for Platform {

}