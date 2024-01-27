use atsamd_hal::{gpio::PinId, ehal::blocking::delay::DelayUs};

use crate::pin::IoPin;

pub struct Onewire<P: PinId, D> {
    delay: D,
    pin: IoPin<P>
}

impl<P: PinId, D: DelayUs<u32>> Onewire<P, D> {
    pub fn new(delay: D, pin: impl Into<IoPin<P>>) -> Self {
        let mut pin = pin.into();
        pin.dirclr();
        pin.outclr();
        pin.pincfg().write(|w| { w.inen().set_bit() });
        pin.enable_sampling();
        Self { delay, pin }
    }

    #[inline]
    fn pin_float(&mut self) {
        self.pin.dirclr();
    }

    #[inline]
    fn pin_low(&mut self) {
        self.pin.dirset();
    }

    #[inline]
    fn pin_read(&mut self) -> bool {
        self.pin.read()
    }

    #[inline(always)]
    fn delay_us(&mut self, us: u32) {
        self.delay.delay_us(us)
    }

    pub fn reset(&mut self) -> bool {
        // wait for bus high
        self.pin_float();
        while self.pin_read() == false {
            self.delay_us(2);
        }

        // reset pulse
        self.pin_low();
        self.delay_us(480);
        self.pin_float();

        // detect presence
        self.delay_us(70);
        let presence = self.pin_read();
        self.delay_us(410);

        presence
    }

    fn write_bit(&mut self, bit: bool) {
        if bit {
            self.pin_low();
            self.delay_us(1);
            self.pin_float();
            self.delay_us(64);
        } else {
            self.pin_low();
            self.delay_us(60);
            self.pin_float();
            self.delay_us(5);
        }
    }

    fn read_bit(&mut self) -> bool {
        self.pin_low();
        self.delay_us(1);
        self.pin_float();
        self.delay_us(13);
        let bit = self.pin_read();
        self.delay_us(50);
        bit
    }

    pub fn write_byte(&mut self, mut byte: u8) {
        for _ in 0..8 {
            self.write_bit(byte & 1 != 0);
            byte >>= 1;
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        let mut byte = 0;
        for _ in 0..8 {
            byte >>= 1;
            byte |= if self.read_bit() { 0x80 } else { 0 };
        }
        byte
    }
}

pub fn ds18b20_read<P: PinId, D: DelayUs<u32>>(bus: &mut Onewire<P, D>) -> u16 {
    bus.reset();
    bus.write_byte(0xCC); // Skip ROM
    bus.write_byte(0x44); // Convert T

    // poll for completion
    while bus.read_byte() == 0 {
        bus.delay_us(10_000);
    }

    bus.reset();
    bus.write_byte(0xCC); // Skip ROM
    bus.write_byte(0xBE); // Read scratchpad
    let tl = bus.read_byte();
    let th = bus.read_byte();

    u16::from_be_bytes([th, tl])
}