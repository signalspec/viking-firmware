#[path = "../../chip/rp/mod.rs"]
mod rp;

use rp::{Gpio, Led};
use zeptos::rp::gpio::*;

pub const PRODUCT_STRING: &'static str = "RP2040 Pico";

pub fn serial_number() -> [u8; 8] {
    zeptos::rp::serial_number()
}

pub fn init() {

}

crate::viking::viking!(
    viking_impl {
        gp0 {
            gpio: Gpio<GPIO00>,
            // spi0_rx
            // i2c0_sda
            // uart0_tx
        }
        gp1 {
            gpio: Gpio<GPIO01>,
            // i2c0_scl
            // uart0_rx
        }
        gp2 {
            gpio: Gpio<GPIO02>,
            // spi0_sck
            // i2c1_sda
        }
        gp3 {
            gpio: Gpio<GPIO03>,
            // spi0_tx
            // i2c1_scl
        }
        gp4 {
            gpio: Gpio<GPIO04>,
            // spi0_rx
            // i2c0_sda
            // uart1_tx
        }
        gp5 {
            gpio: Gpio<GPIO05>,
            // i2c0_scl
            // uart1_rx
        }
        gp6 {
            gpio: Gpio<GPIO06>,
            // spi0_sck
            // i2c1_sda
        }
        gp7 {
            gpio: Gpio<GPIO07>,
            // spi0_tx
            // i2c1_scl
        }
        gp8 {
            gpio: Gpio<GPIO08>,
            // spi1_rx
            // i2c0_sda
            // uart1_tx
        }
        gp9 {
            gpio: Gpio<GPIO09>,
            // i2c0_scl
            // uart1_rx
        }
        gp10 {
            gpio: Gpio<GPIO10>,
            // spi1_sck
            // i2c1_sda
        }
        gp11 {
            gpio: Gpio<GPIO11>,
            // spi1_tx
            // i2c1_scl
        }
        gp12 {
            gpio: Gpio<GPIO12>,
            // spi1_rx
            // i2c0_sda
            // uart0_tx
        }
        gp13 {
            gpio: Gpio<GPIO13>,
            // i2c0_scl
            // uart0_rx
        }
        gp14 {
            gpio: Gpio<GPIO14>,
            // spi1_sck
            // i2c1_sda
        }
        gp15 {
            gpio: Gpio<GPIO15>,
            // spi1_tx
            // i2c1_scl
        }

        gp16 {
            gpio: Gpio<GPIO16>,
            // spi0_rx
            // i2c0_sda
            // uart0_tx
        }
        gp17 {
            gpio: Gpio<GPIO17>,
            // i2c0_scl
            // uart0_rx
        }
        gp18 {
            gpio: Gpio<GPIO18>,
            // spi0_sck
            // i2c1_sda
        }
        gp19 {
            gpio: Gpio<GPIO19>,
            // spi0_tx
            // i2c1_scl
        }
        gp20 {
            gpio: Gpio<GPIO20>,
            // i2c0_sda
        }
        gp21 {
            gpio: Gpio<GPIO21>,
            // i2c0_scl
        }
        gp22 {
            gpio: Gpio<GPIO22>,
        }
        gp26 {
            gpio: Gpio<GPIO26>,
            // adc0
            // i2c1_sda
        }
        gp27 {
            gpio: Gpio<GPIO27>,
            // adc1
            // i2c1_scl
        }
        gp28 {
            gpio: Gpio<GPIO28>,
            // adc2
        }

        led {
            led: Led<GPIO25, true, { viking_protocol::protocol::led::binary::color::GREEN }>,
        }

        spi0 {

        }
        spi1 {

        }

        i2c0 {

        }
        i2c1 {

        }

        uart0 {

        }
        uart1 {

        }
    }
);
