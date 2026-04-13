# Viking-firwmare

Implementation of [Viking](https://github.com/signalspec/viking) for:
 
  * [Raspberry Pi RP2040 Pico](./board/rp2040-pico)
  * [Raspberry Pi RP2350 Pico 2](./board/rp2350-pico2)
  * [Arduino Zero](./board/samd21-arduino-zero)
  * [Atmel SAM D21 Xplained Pro](./board/samd21-xplained)
  
The firmware is written in Rust on top of the [Zeptos](https://github.com/kevinmehall/zeptos) async runtime and USB stack.

To build the firmware, run `cargo xtask dist` to build all firmware images, or switch to a `board/<board-name>` directory and use the standard `cargo build`. Zeptos requires Rust nightly.
