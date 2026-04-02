#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![feature(macro_metavar_expr)]

use zeptos::rp::gpio::*;
use zeptos::rp::i2c::{I2c0, I2c1};

const PRODUCT_STRING: &'static str = "RP2040 Pico (Viking)";
const CMD_BUF_SIZE: usize = 64 * 1024;
const RES_BUF_SIZE: usize = 64 * 1024;
const EVT_BUF_SIZE: usize = 64 * 1024;

mod common;
mod rp;
use rp::{Gpio, LevelInterrupt, Led, I2c, I2cSdaPin, I2cSclPin, Platform};

#[zeptos::main]
async fn main(rt: zeptos::Runtime, hw: zeptos::Hardware) {
    let (usb, platform) = Platform::new(rt, hw);
    crate::common::run(usb, platform).await;
}

viking!{
    resource gp0 {
        gpio: Gpio<GPIO00>,
        level_int: LevelInterrupt<GPIO00>,
        i2c0_sda: I2cSdaPin<GPIO00, I2c0>,
        // spi0_rx
        // uart0_tx
    }

    resource gp1 {
        gpio: Gpio<GPIO01>,
        level_int: LevelInterrupt<GPIO01>,
        i2c0_scl: I2cSclPin<GPIO01, I2c0>,
        // uart0_rx
    }

    resource gp2 {
        gpio: Gpio<GPIO02>,
        level_int: LevelInterrupt<GPIO02>,
        i2c1_sda: I2cSdaPin<GPIO02, I2c1>,
        // spi0_sck
    }

    resource gp3 {
        gpio: Gpio<GPIO03>,
        level_int: LevelInterrupt<GPIO03>,
        i2c1_scl: I2cSclPin<GPIO03, I2c1>,

        // spi0_tx
    }

    resource gp4 {
        gpio: Gpio<GPIO04>,
        level_int: LevelInterrupt<GPIO04>,
        i2c0_sda: I2cSdaPin<GPIO04, I2c0>,

        // spi0_rx
        // uart1_tx
    }

    resource gp5 {
        gpio: Gpio<GPIO05>,
        level_int: LevelInterrupt<GPIO05>,
        i2c0_scl: I2cSclPin<GPIO05, I2c0>,

        // uart1_rx
    }

    resource gp6 {
        gpio: Gpio<GPIO06>,
        level_int: LevelInterrupt<GPIO06>,
        i2c1_sda: I2cSdaPin<GPIO06, I2c1>,

        // spi0_sck
    }

    resource gp7 {
        gpio: Gpio<GPIO07>,
        level_int: LevelInterrupt<GPIO07>,
        i2c1_scl: I2cSclPin<GPIO07, I2c1>,

        // spi0_tx
    }

    resource gp8 {
        gpio: Gpio<GPIO08>,
        level_int: LevelInterrupt<GPIO08>,
        i2c0_sda: I2cSdaPin<GPIO08, I2c0>,

        // spi1_rx
        // uart1_tx
    }

    resource gp9 {
        gpio: Gpio<GPIO09>,
        level_int: LevelInterrupt<GPIO09>,
        i2c0_scl: I2cSclPin<GPIO09, I2c0>,

        // uart1_rx
    }

    resource gp10 {
        gpio: Gpio<GPIO10>,
        level_int: LevelInterrupt<GPIO10>,
        i2c1_sda: I2cSdaPin<GPIO10, I2c1>,

        // spi1_sck
    }

    resource gp11 {
        gpio: Gpio<GPIO11>,
        i2c1_scl: I2cSclPin<GPIO11, I2c1>,
        // spi1_tx
    }

    resource gp12 {
        gpio: Gpio<GPIO12>,
        level_int: LevelInterrupt<GPIO12>,
        i2c0_sda: I2cSdaPin<GPIO12, I2c0>,

        // spi1_rx
        // uart0_tx
    }

    resource gp13 {
        gpio: Gpio<GPIO13>,
        level_int: LevelInterrupt<GPIO13>,
        i2c0_scl: I2cSclPin<GPIO13, I2c0>,

        // uart0_rx
    }

    resource gp14 {
        gpio: Gpio<GPIO14>,
        level_int: LevelInterrupt<GPIO14>,
        i2c1_sda: I2cSdaPin<GPIO14, I2c1>,

        // spi1_sck
    }

    resource gp15 {
        gpio: Gpio<GPIO15>,
        level_int: LevelInterrupt<GPIO15>,
        i2c1_scl: I2cSclPin<GPIO15, I2c1>,

        // spi1_tx
    }

    resource gp16 {
        gpio: Gpio<GPIO16>,
        level_int: LevelInterrupt<GPIO16>,
        i2c0_sda: I2cSdaPin<GPIO16, I2c0>,

        // spi0_rx
        // uart0_tx
    }

    resource gp17 {
        gpio: Gpio<GPIO17>,
        level_int: LevelInterrupt<GPIO17>,
        i2c0_scl: I2cSclPin<GPIO17, I2c0>,

        // uart0_rx
    }

    resource gp18 {
        gpio: Gpio<GPIO18>,
        level_int: LevelInterrupt<GPIO18>,
        i2c1_sda: I2cSdaPin<GPIO18, I2c1>,

        // spi0_sck
    }

    resource gp19 {
        gpio: Gpio<GPIO19>,
        level_int: LevelInterrupt<GPIO19>,
        i2c1_scl: I2cSclPin<GPIO19, I2c1>,

        // spi0_tx
    }

    resource gp20 {
        gpio: Gpio<GPIO20>,
        level_int: LevelInterrupt<GPIO20>,
        i2c0_sda: I2cSdaPin<GPIO20, I2c0>,
    }

    resource gp21 {
        gpio: Gpio<GPIO21>,
        level_int: LevelInterrupt<GPIO21>,
        i2c0_scl: I2cSclPin<GPIO21, I2c0>,
    }

    resource gp22 {
        gpio: Gpio<GPIO22>,
        level_int: LevelInterrupt<GPIO22>,
    }

    resource gp26 {
        gpio: Gpio<GPIO26>,
        level_int: LevelInterrupt<GPIO26>,
        i2c1_sda: I2cSdaPin<GPIO26, I2c1>,

        // adc0
    }

    resource gp27 {
        gpio: Gpio<GPIO27>,
        level_int: LevelInterrupt<GPIO27>,
        i2c1_scl: I2cSclPin<GPIO27, I2c1>,

        // adc1
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
