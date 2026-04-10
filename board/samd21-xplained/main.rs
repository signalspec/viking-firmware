#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(macro_metavar_expr)]

use zeptos::samd::{gpio::{alternate::*, *}, pac::Interrupt, sercom::{Sercom0, Sercom1, Sercom2, Sercom5}};

const PRODUCT_STRING: &'static str = "SAM D21 Xplained (Viking)";
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
    // Shared I2C pins
    resource pa08 {
        gpio: sam0::Gpio<PA08>,
        sercom2_sda: sam0::SercomSCLPin<PA08, Sercom2, C>,
    }
    resource pa09 {
        gpio: sam0::Gpio<PA09>,
        sercom2_scl: sam0::SercomSDAPin<PA09, Sercom2, C>,
    }

    // EXT1 Pins
    resource pb00 {
        gpio: sam0::Gpio<PB00>,
        //ain8
    }
    resource pb01 {
        gpio: sam0::Gpio<PB01>,
        //ain9
    }
    resource pb06 {
        gpio: sam0::Gpio<PB06>,
    }
    resource pb07 {
        gpio: sam0::Gpio<PB07>,
    }
    resource pb04 {
        gpio: sam0::Gpio<PB04>,
        level_interrupt: sam0::LevelInterrupt<PB04, 4>,
    }
    resource pb05 {
        gpio: sam0::Gpio<PB05>,
    }
    //pa09, pa09 shared
    resource pb09 {
        gpio: sam0::Gpio<PB09>,
        // sercom4 UART RX
    }
    resource pb08 {
        gpio: sam0::Gpio<PB08>,
        // sercom4 UART TX
    }
    resource pa05 {
        gpio: sam0::Gpio<PA05>, // CS
    }
    resource pa06 {
        gpio: sam0::Gpio<PA06>,
        sercom0_sdo: sam0::SercomSDOPin<PA06, Sercom0, D>, // [sercom pad 2]
    }
    resource pa04 {
        gpio: sam0::Gpio<PA04>,
        sercom0_sdi: sam0::SercomSDIPin<PA04, Sercom0, D>, // [sercom pad 0]
    }
    resource pa07 {
        gpio: sam0::Gpio<PA07>,
        sercom0_sck: sam0::SercomSCKPin<PA07, Sercom0, D>, // [sercom pad 3]
    }

    // EXT2 Pins
    resource pa10 {
        gpio: sam0::Gpio<PA10>,
        //ain18
    }
    resource pa11 {
        gpio: sam0::Gpio<PA11>,
        //ain19
    }
    resource pa20 {
        gpio: sam0::Gpio<PA20>,
    }
    resource pa21 {
        gpio: sam0::Gpio<PA21>,
    }
    resource pb12 {
        gpio: sam0::Gpio<PB12>,
    }
    resource pb13 {
        gpio: sam0::Gpio<PB13>,
    }
    resource pb14 {
        gpio: sam0::Gpio<PB14>,
        level_interrupt: sam0::LevelInterrupt<PB14, 14>,
    }
    resource pb15 {
        gpio: sam0::Gpio<PB15>,
    }
    // pa08, pa09 shared
    resource pb11 {
        gpio: sam0::Gpio<PB11>,
        // sercom4 UART RX
    }
    resource pb10 {
        gpio: sam0::Gpio<PB10>,
        // sercom4 UART TX
    }
    resource pa17 {
        gpio: sam0::Gpio<PA17>, // CS
    }
    resource pa18 {
        gpio: sam0::Gpio<PA18>,
        sercom1_sdo: sam0::SercomSDOPin<PA18, Sercom1, C>, // [sercom pad 2]
    }
    resource pa16 {
        gpio: sam0::Gpio<PA16>,
        sercom1_sdi: sam0::SercomSDIPin<PA16, Sercom1, C>, // [sercom pad 0]
    }
    resource pa19 {
        gpio: sam0::Gpio<PA19>,
        sercom1_sck: sam0::SercomSCKPin<PA19, Sercom1, C>, // [sercom pad 3]
    }

    // EXT3 Pins
    resource pa02 {
        gpio: sam0::Gpio<PA02>,
        // AIN0
    }
    resource pa03 {
        gpio: sam0::Gpio<PA03>,
        // AIN1
    }
    resource led { // PB30
        led: sam0::Led<PB30, false, { viking_protocol::protocol::led::binary::color::AMBER }>,
    }
    resource pa15 {
        gpio: sam0::Gpio<PA15>, // SW0
    }
    resource pa12 {
        gpio: sam0::Gpio<PA12>,
    }
    resource pa13 {
        gpio: sam0::Gpio<PA13>, // Serial flash CS, not connected to EXT3 by default
    }
    resource pa28 {
        gpio: sam0::Gpio<PA28>,
        level_interrupt: sam0::LevelInterrupt<PA28, 8>,
    }
    resource pa27 {
        gpio: sam0::Gpio<PA27>,
    }
    // pa08, pa09, pb11, pb10 shared
    resource pb17 {
        gpio: sam0::Gpio<PB17>,
    }
    resource pb22 {
        gpio: sam0::Gpio<PB22>,
        sercom5_sdo: sam0::SercomSDOPin<PB22, Sercom5, D>, // [sercom pad 2] serial flash
    }
    resource pb16 {
        gpio: sam0::Gpio<PB16>,
        sercom5_sdi: sam0::SercomSDIPin<PB16, Sercom5, C>, // [sercom pad 0] serial flash
    }
    resource pb23 {
        gpio: sam0::Gpio<PB23>,
        sercom5_sck: sam0::SercomSCKPin<PB23, Sercom5, D>, // [sercom pad 3] serial flash
    }

    resource sercom0 {
        spi: sam0::SercomSPI<Sercom0, 1, 0, true>,
    }
    resource sercom1 {
        spi: sam0::SercomSPI<Sercom1, 1, 0, true>,
    }
    resource sercom2 {
        i2c: sam0::SercomI2C<Sercom2, true>,
    }
    resource sercom5 {
        spi: sam0::SercomSPI<Sercom5, 1, 0, true>,
    }
}
