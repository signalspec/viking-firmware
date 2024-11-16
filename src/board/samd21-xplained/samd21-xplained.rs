#[path = "../../chip/sam0/mod.rs"]
mod sam0;

use sam0::Sercom0;
use zeptos::samd::{gpio::{alternate::*, *}, pac::Interrupt};

pub const PRODUCT_STRING: &'static str = "SAM D21 Xplained";
pub use zeptos::samd::serial_number;

pub fn init() {
    let pm = unsafe { zeptos::samd::pac::PM::steal() };
    let mut gclk = unsafe { zeptos::samd::pac::GCLK::steal() };
    let eic = unsafe { zeptos::samd::pac::EIC::steal() };

    pm.apbcmask.write(|w| {
        w.sercom0_().set_bit();
        w.sercom1_().set_bit()
    });

    eic.ctrl.write(|w| w.enable().set_bit());

    zeptos::samd::clock::enable_clock(&mut gclk, zeptos::samd::pac::gclk::clkctrl::IDSELECT_A::SERCOM0_CORE, zeptos::samd::pac::gclk::clkctrl::GENSELECT_A::GCLK0);
    zeptos::samd::clock::enable_clock(&mut gclk, zeptos::samd::pac::gclk::clkctrl::IDSELECT_A::SERCOM1_CORE, zeptos::samd::pac::gclk::clkctrl::GENSELECT_A::GCLK0);

    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::SERCOM0);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EIC);
    }
}

crate::viking::viking!(
    viking_impl {
        pa08 {
            gpio: sam0::Gpio<PA08>,
            sercom0_i2c_sda: sam0::SercomSCLPin<PA08, Sercom0, C>,
            sercom0_spi_so: sam0::SercomSOPin<PA08, Sercom0, C>,
        }
        pa09 {
            gpio: sam0::Gpio<PA09>,
            sercom0_i2c_scl: sam0::SercomSDAPin<PA09, Sercom0, C>,
            sercom0_spi_sck: sam0::SercomSCKPin<PA09, Sercom0, C>,
        }
        pa10 {
            gpio: sam0::Gpio<PA10>,
            sercom0_spi_si: sam0::SercomSIPin<PA10, Sercom0, C>,
        }
        pa11 {
            gpio: sam0::Gpio<PA11>,
            level_interrupt: sam0::LevelInterrupt<PA11, 11>,
        }
        pb30 {
            gpio: sam0::Gpio<PB30>,
        }
        sercom0 {
            i2c: sam0::SercomI2C<Sercom0>,
            spi: sam0::SercomSPI<Sercom0, 0, 2>,
        }
    }
);
