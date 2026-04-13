# RP2350 Pico 2

## Installation

 1. Download the RP2350 Pico2 `.uf2` release from the [releases page](https://github.com/signalspec/viking-firmware/releases).
 2. Enter the Pico's bootloader mode by holding the "BOOTSEL" button while plugging it into USB.
 3. The Pico should appear as a USB mass storage device. Copy the `.uf2` file to the device to flash the firmware. Alternatively, flash with `picotool load viking-firmware-rp2350-pico2.uf2 && picotool reboot`.
 4. The Pico will reboot and should be ready to use with Viking.


## Pinout and Resources

![Pico pinout](https://media.signalspec.org/viking/pico-pinout.png)

```
gp0
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c0_sda"
    spi_sdi_pin "spi0_sdi"
gp1
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c0_scl"
gp2
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c1_sda"
    spi_sck_pin "spi0_sck"
gp3
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c1_scl"
    spi_sdo_pin "spi0_sdo"
gp4
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c0_sda"
    spi_sdi_pin "spi0_sdi"
gp5
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c0_scl"
gp6
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c1_sda"
    spi_sck_pin "spi0_sck"
gp7
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c1_scl"
    spi_sdo_pin "spi0_sdo"
gp8
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c0_sda"
    spi_sdi_pin "spi1_sdi"
gp9
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c0_scl"
gp10
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c1_sda"
    spi_sck_pin "spi1_sck"
gp11
    gpio_pin "gpio"
    i2c_sda_pin "i2c1_scl"
    spi_sdo_pin "spi1_sdo"
gp12
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c0_sda"
    spi_sdi_pin "spi1_sdi"
gp13
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c0_scl"
gp14
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c1_sda"
    spi_sck_pin "spi1_sck"
gp15
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c1_scl"
    spi_sdo_pin "spi1_sdo"
gp16
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c0_sda"
    spi_sdi_pin "spi0_sdi"
gp17
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c0_scl"
gp18
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c1_sda"
    spi_sck_pin "spi0_sck"
gp19
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c1_scl"
    spi_sdo_pin "spi0_sdo"
gp20
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c0_sda"
gp21
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c0_scl"
gp22
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
gp26
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_scl_pin "i2c1_sda"
gp27
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
    i2c_sda_pin "i2c1_scl"
gp28
    gpio_pin "gpio"
    gpio_level_interrupt "level_int"
led
    led "led"
spi0
    spi_controller "controller"
spi1
    spi_controller "controller"
i2c0
    i2c_controller "controller"
i2c1
    i2c_controller "controller"
```
