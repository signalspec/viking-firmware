#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(macro_metavar_expr)]

use viking_firmware_common::sam0::{ self, Sercom0, Platform };
use zeptos::samd::{gpio::{alternate::*, *}, pac::Interrupt};

#[zeptos::main]
async fn main(rt: zeptos::Runtime, hw: zeptos::Hardware) {
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

    let (usb, platform) = Platform::new(rt, hw);
    viking_impl::run(usb, platform).await;
}


viking_firmware_common::viking!(
    viking_impl<Platform> {
        const PRODUCT_STRING: &'static str = "SAM D21 Xplained";
        const CMD_BUF_SIZE: usize = 8192;
        const RES_BUF_SIZE: usize = 8192;
        const EVT_BUF_SIZE: usize = 4096;

        resource pa08 {
            gpio: sam0::Gpio<PA08>,
            sercom0_i2c_sda: sam0::SercomSCLPin<PA08, Sercom0, C>,
            sercom0_spi_so: sam0::SercomSDOPin<PA08, Sercom0, C>,
        }
        resource pa09 {
            gpio: sam0::Gpio<PA09>,
            sercom0_i2c_scl: sam0::SercomSDAPin<PA09, Sercom0, C>,
            sercom0_spi_sck: sam0::SercomSCKPin<PA09, Sercom0, C>,
        }
        resource pa10 {
            gpio: sam0::Gpio<PA10>,
            sercom0_spi_si: sam0::SercomSDIPin<PA10, Sercom0, C>,
        }
        resource pa11 {
            gpio: sam0::Gpio<PA11>,
            level_interrupt: sam0::LevelInterrupt<PA11, 11>,
        }
        resource led {
            led: sam0::Led<PB30, false, { viking_protocol::protocol::led::binary::color::AMBER }>,
        }
        resource sercom0 {
            i2c: sam0::SercomI2C<Sercom0>,
            spi: sam0::SercomSPI<Sercom0, 0, 2>,
        }
    }
);
