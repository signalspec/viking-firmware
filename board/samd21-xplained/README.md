# SAM D21 Xplained Pro

## Installation

Download the SAM D21 Xplained `.elf` release from the [releases page](https://github.com/signalspec/viking-firmware/releases).

Connect the USB port labeled "DEBUG USB", and flash the firmware with probe-rs and the onboard CMSIS-DAP probe:

```bash
probe-rs download --probe 03eb:2111 --chip ATSAMD21J18A dist/viking-firmware-samd21-xplained.elf --preverify --restore-unwritten 
```

Then, switch to the "TARGET USB" port to use Viking.

## Pinout and Resources

[User Guide](https://ww1.microchip.com/downloads/aemDocuments/documents/OTH/ProductDocuments/UserGuides/Atmel-42220-SAMD21-Xplained-Pro_User-Guide.pdf)

```
pa08
    gpio_pin "gpio"
    i2c_sda_pin "sercom2_sda"
pa09
    gpio_pin "gpio"
    i2c_scl_pin "sercom2_scl"
pb00
    gpio_pin "gpio"
pb01
    gpio_pin "gpio"
pb06
    gpio_pin "gpio"
pb07
    gpio_pin "gpio"
pb04
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
pb05
    gpio_pin "gpio"
pb09
    gpio_pin "gpio"
pb08
    gpio_pin "gpio"
pa05
    gpio_pin "gpio"
pa06
    gpio_pin "gpio"
    spi_sdo_pin "sercom0_sdo"
pa04
    gpio_pin "gpio"
    spi_sdi_pin "sercom0_sdi"
pa07
    gpio_pin "gpio"
    spi_sck_pin "sercom0_sck"
pa10
    gpio_pin "gpio"
pa11
    gpio_pin "gpio"
pa20
    gpio_pin "gpio"
pa21
    gpio_pin "gpio"
pb12
    gpio_pin "gpio"
pb13
    gpio_pin "gpio"
pb14
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
pb15
    gpio_pin "gpio"
pb11
    gpio_pin "gpio"
pb10
    gpio_pin "gpio"
pa17
    gpio_pin "gpio"
pa18
    gpio_pin "gpio"
    spi_sdo_pin "sercom1_sdo"
pa16
    gpio_pin "gpio"
    spi_sdi_pin "sercom1_sdi"
pa19
    gpio_pin "gpio"
    spi_sck_pin "sercom1_sck"
pa02
    gpio_pin "gpio"
pa03
    gpio_pin "gpio"
led
    led "led"
pa15
    gpio_pin "gpio"
pa12
    gpio_pin "gpio"
pa13
    gpio_pin "gpio"
pa28
    gpio_pin "gpio"
    gpio_level_interrupt "level_interrupt"
pa27
    gpio_pin "gpio"
pb17
    gpio_pin "gpio"
pb22
    gpio_pin "gpio"
    spi_sdo_pin "sercom5_sdo"
pb16
    gpio_pin "gpio"
    spi_sdi_pin "sercom5_sdi"
pb23
    gpio_pin "gpio"
    spi_sck_pin "sercom5_sck"
sercom0
    spi_controller "spi"
sercom1
    spi_controller "spi"
sercom2
    i2c_controller "i2c"
sercom5
    spi_controller "spi"
```
