#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(macro_metavar_expr, macro_metavar_expr_concat)]

use zeptos::rp::gpio::*;

const PRODUCT_STRING: &'static str = "RP2350 Pico 2 (Viking)";
const CMD_BUF_SIZE: usize = 64 * 1024;
const RES_BUF_SIZE: usize = 64 * 1024;
const EVT_BUF_SIZE: usize = 64 * 1024;

mod common;
mod rp;
use rp::{Gpio, LevelInterrupt, Led, Platform};

#[zeptos::main]
async fn main(rt: zeptos::Runtime, hw: zeptos::Hardware) {
    let (usb, platform) = Platform::new(rt, hw);
    common::run(usb, platform).await;
}

viking!{
    resource gp0 {
        gpio: Gpio<GPIO00>,
        level_int: LevelInterrupt<GPIO00>,
        // spi0_rx
        // i2c0_sda
        // uart0_tx
    }

    resource gp1 {
        gpio: Gpio<GPIO01>,
        level_int: LevelInterrupt<GPIO01>,

        // i2c0_scl
        // uart0_rx
    }

    resource gp2 {
        gpio: Gpio<GPIO02>,
        level_int: LevelInterrupt<GPIO02>,

        // spi0_sck
        // i2c1_sda
    }

    resource gp3 {
        gpio: Gpio<GPIO03>,
        level_int: LevelInterrupt<GPIO03>,

        // spi0_tx
        // i2c1_scl
    }

    resource gp4 {
        gpio: Gpio<GPIO04>,
        level_int: LevelInterrupt<GPIO04>,

        // spi0_rx
        // i2c0_sda
        // uart1_tx
    }

    resource gp5 {
        gpio: Gpio<GPIO05>,
        level_int: LevelInterrupt<GPIO05>,

        // i2c0_scl
        // uart1_rx
    }

    resource gp6 {
        gpio: Gpio<GPIO06>,
        level_int: LevelInterrupt<GPIO06>,

        // spi0_sck
        // i2c1_sda
    }

    resource gp7 {
        gpio: Gpio<GPIO07>,
        level_int: LevelInterrupt<GPIO07>,

        // spi0_tx
        // i2c1_scl
    }

    resource gp8 {
        gpio: Gpio<GPIO08>,
        level_int: LevelInterrupt<GPIO08>,

        // spi1_rx
        // i2c0_sda
        // uart1_tx
    }

    resource gp9 {
        gpio: Gpio<GPIO09>,
        level_int: LevelInterrupt<GPIO09>,

        // i2c0_scl
        // uart1_rx
    }

    resource gp10 {
        gpio: Gpio<GPIO10>,
        level_int: LevelInterrupt<GPIO10>,

        // spi1_sck
        // i2c1_sda
    }

    resource gp11 {
        gpio: Gpio<GPIO11>,
        // spi1_tx
        // i2c1_scl
    }

    resource gp12 {
        gpio: Gpio<GPIO12>,
        level_int: LevelInterrupt<GPIO12>,

        // spi1_rx
        // i2c0_sda
        // uart0_tx
    }

    resource gp13 {
        gpio: Gpio<GPIO13>,
        level_int: LevelInterrupt<GPIO13>,

        // i2c0_scl
        // uart0_rx
    }

    resource gp14 {
        gpio: Gpio<GPIO14>,
        level_int: LevelInterrupt<GPIO14>,

        // spi1_sck
        // i2c1_sda
    }

    resource gp15 {
        gpio: Gpio<GPIO15>,
        level_int: LevelInterrupt<GPIO15>,

        // spi1_tx
        // i2c1_scl
    }

    resource gp16 {
        gpio: Gpio<GPIO16>,
        level_int: LevelInterrupt<GPIO16>,

        // spi0_rx
        // i2c0_sda
        // uart0_tx
    }

    resource gp17 {
        gpio: Gpio<GPIO17>,
        level_int: LevelInterrupt<GPIO17>,

        // i2c0_scl
        // uart0_rx
    }

    resource gp18 {
        gpio: Gpio<GPIO18>,
        level_int: LevelInterrupt<GPIO18>,

        // spi0_sck
        // i2c1_sda
    }

    resource gp19 {
        gpio: Gpio<GPIO19>,
        level_int: LevelInterrupt<GPIO19>,

        // spi0_tx
        // i2c1_scl
    }

    resource gp20 {
        gpio: Gpio<GPIO20>,
        level_int: LevelInterrupt<GPIO20>,

        // i2c0_sda
    }

    resource gp21 {
        gpio: Gpio<GPIO21>,
        level_int: LevelInterrupt<GPIO21>,

        // i2c0_scl
    }

    resource gp22 {
        gpio: Gpio<GPIO22>,
        level_int: LevelInterrupt<GPIO22>,

    }

    resource gp26 {
        gpio: Gpio<GPIO26>,
        level_int: LevelInterrupt<GPIO26>,

        // adc0
        // i2c1_sda
    }

    resource gp27 {
        gpio: Gpio<GPIO27>,
        level_int: LevelInterrupt<GPIO27>,

        // adc1
        // i2c1_scl
    }

    resource gp28 {
        gpio: Gpio<GPIO28>,
        level_int: LevelInterrupt<GPIO28>,

        // adc2
    }

    resource led {
        led: Led<GPIO25, true, { viking_protocol::protocol::led::binary::color::GREEN }>,
    }

    resource spi0 {

    }

    resource spi1 {

    }

    resource i2c0 {

    }

    resource i2c1 {

    }

    resource uart0 {

    }

    resource uart1 {

    }
}
