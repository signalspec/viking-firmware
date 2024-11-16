#[path = "../../chip/sam0/mod.rs"]
mod sam0;

use sam0::{Sercom0, Sercom1 };
use zeptos::samd::{gpio::{alternate::*, *}, pac::Interrupt};

pub const PRODUCT_STRING: &'static str = "Seahorse";
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
        cortex_m::peripheral::NVIC::unmask(Interrupt::SERCOM1);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EIC);
    }

    // SPI fixed pins
    PA07::set_alternate(Alternate::C); // SCK
    PA08::set_alternate(Alternate::D); // SI
    PA09::set_alternate(Alternate::D); // SO

    // I2C fixed pins
    PA22::set_alternate(Alternate::C); // SDA
    PA23::set_alternate(Alternate::C); // SCL

}

crate::viking::viking!(
    viking_impl {
        ce {
            gpio: sam0::Gpio<PA02>,
        }

        cs {
            gpio: sam0::Gpio<PA06>,
        }

        irq {
            gpio: sam0::Gpio<PA16>,
            level_interrupt: sam0::LevelInterrupt<PA16, 0>,
        }

        spi {
            spi: sam0::SercomSPI<Sercom0, 2, 2>,
        }

        i2c {
            i2c: sam0::SercomI2C<Sercom1>,
        }
    }
);
