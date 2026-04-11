#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(macro_metavar_expr)]

use zeptos::samd::{gpio::{alternate::*, *}, pac::Interrupt, sercom::{Sercom1, Sercom3}};

const PRODUCT_STRING: &'static str = "Arduino Zero (Viking)";
const CMD_BUF_SIZE: usize = 8192;
const RES_BUF_SIZE: usize = 8192;
const EVT_BUF_SIZE: usize = 4096;

mod common;
mod sam0;
use sam0::Platform;

#[zeptos::main]
async fn main(rt: zeptos::Runtime, hw: zeptos::Hardware) {
    let eic = unsafe { zeptos::samd::pac::EIC::steal() };

    eic.ctrl.write(|w| w.enable().set_bit());

    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::EIC);
    }

    let (usb, platform) = Platform::new(rt, hw);
    common::run(usb, platform).await;
}


viking!{
    resource d0 {
        gpio: sam0::Gpio<PA11>,
        level_interrupt: sam0::LevelInterrupt<PA11, 11>,
        // UART TX
    }
    resource d1 {
        gpio: sam0::Gpio<PA10>,
        level_interrupt: sam0::LevelInterrupt<PA10, 10>,
        // UART RX
    }
    resource d2 {
        gpio: sam0::Gpio<PA14>,
        level_interrupt: sam0::LevelInterrupt<PA14, 14>,
    }
    resource d3 {
        gpio: sam0::Gpio<PA09>,
        level_interrupt: sam0::LevelInterrupt<PA09, 9>,
    }
    resource d4 {
        gpio: sam0::Gpio<PA08>,
    }
    resource d5 {
        gpio: sam0::Gpio<PA15>,
        level_interrupt: sam0::LevelInterrupt<PA15, 15>,
    }
    resource d6 {
        gpio: sam0::Gpio<PA20>,
        level_interrupt: sam0::LevelInterrupt<PA20, 4>,
    }
    resource d7 {
        gpio: sam0::Gpio<PA21>,
        level_interrupt: sam0::LevelInterrupt<PA21, 5>,
    }
    resource d8 {
        gpio: sam0::Gpio<PA06>,
        level_interrupt: sam0::LevelInterrupt<PA06, 6>,
    }
    resource d9 {
        gpio: sam0::Gpio<PA07>,
        level_interrupt: sam0::LevelInterrupt<PA07, 7>,
    }
    resource d10 {
        gpio: sam0::Gpio<PA18>,
        level_interrupt: sam0::LevelInterrupt<PA18, 2>,
    }
    resource d11 {
        gpio: sam0::Gpio<PA16>,
        level_interrupt: sam0::LevelInterrupt<PA16, 0>,
        sdo: sam0::SercomSDOPin<PA16, Sercom1, C>, // [sercom1 pad 0]
    }
    resource d12 {
        gpio: sam0::Gpio<PA19>,
        level_interrupt: sam0::LevelInterrupt<PA19, 3>,
        sdi: sam0::SercomSDIPin<PA19, Sercom1, C>, // [sercom1 pad 3]
    }
    resource d13 {
        gpio: sam0::Gpio<PA17>,
        level_interrupt: sam0::LevelInterrupt<PA17, 1>,
        sck: sam0::SercomSCKPin<PA17, Sercom1, C>, // [sercom1 pad 1]
    }

    resource a0 {
        gpio: sam0::Gpio<PA02>,
    }
    resource a1 {
        gpio: sam0::Gpio<PB08>,
    }
    resource a2 {
        gpio: sam0::Gpio<PB09>,
    }
    resource a3 {
        gpio: sam0::Gpio<PA04>,
    }
    resource a4 {
        gpio: sam0::Gpio<PA05>,
    }
    resource a5 {
        gpio: sam0::Gpio<PB02>,
    }

    resource sda {
        gpio: sam0::Gpio<PA22>,
        sda: sam0::SercomSDAPin<PA22, Sercom3, C>,
    }
    resource scl {
        gpio: sam0::Gpio<PA23>,
        scl: sam0::SercomSCLPin<PA23, Sercom3, C>,
    }

    resource spi {
        spi: sam0::SercomSPI<Sercom1, 0, 3, true>,
    }
    resource i2c {
        i2c: sam0::SercomI2C<Sercom3, true>,
    }

    resource led {
        led: sam0::Led<PA17, true, { viking_protocol::protocol::led::binary::color::GREEN }>,
    }

    resource led_tx {
        led: sam0::Led<PA27, false, { viking_protocol::protocol::led::binary::color::YELLOW }>,
    }

    resource led_rx {
        led: sam0::Led<PB03, false, { viking_protocol::protocol::led::binary::color::YELLOW }>,
    }
}
