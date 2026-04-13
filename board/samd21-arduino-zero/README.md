# Arduino Zero

## Installation

Download the Arduino Zero `.bin` or `.elf` release from the [releases page](https://github.com/signalspec/viking-firmware/releases).

The firmware is linked to be flashed at address `0x2000`, after the Arduino bootloader.

### With Arduino bootloader and `bossac`

Connect the target USB port next to the reset button, and enter bootloader mode by double-pressing the reset button. Then, flash the firmware with:

```bash
bossac -p /dev/ttyACM0 -e -w -v -R --offset=0x2000 viking-firmware-samd21-arduino-zero.bin
```

### With `probe-rs` and the onboard CMSIS-DAP probe

Connect the USB port labeled "DEBUG", and flash the firmware with:

```bash
probe-rs download --probe 03eb:2157 --chip ATSAMD21J18A viking-firmware-samd21-arduino-zero.elf --preverify --restore-unwritten 
```

Then, switch to the target USB port to use Viking.

## Pinout and Resources

![Arduino Zero pinout](https://media.signalspec.org/viking/arduino-zero-pinout.png)

```
d0
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d1
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d2
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d3
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d4
    gpio_pin "gpio"
d5
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d6
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d7
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d8
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d9
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d10
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
d11
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
    spi_sdo_pin "sdo"
d12
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
    spi_sdi_pin "sdi"
d13
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
    spi_sck_pin "sck"
a0
    gpio_pin "gpio"
a1
    gpio_pin "gpio"
a2
    gpio_pin "gpio"
a3
    gpio_pin "gpio"
a4
    gpio_pin "gpio"
a5
    gpio_pin "gpio"
sda
    gpio_pin "gpio"
    i2c_scl_pin "sda"
scl
    gpio_pin "gpio"
    i2c_sda_pin "scl"
spi
    spi_controller "spi"
i2c
    i2c_controller "i2c"
led
    led "led"
led_tx
    led "led"
led_rx
    led "led"
```
