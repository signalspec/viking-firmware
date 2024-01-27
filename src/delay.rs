//! Delays
//! Based on atsamd-rs under MIT or Apache-2.0

use core::marker::PhantomData;

use cortex_m::peripheral::SYST;

use atsamd_hal::ehal::blocking::delay::{DelayMs, DelayUs};

/// System timer (SysTick) as a delay provider
pub struct Delay<const CLOCK_HZ: u32> {
    _no_send: PhantomData<*const ()>,
}

impl<const CLOCK_HZ: u32> Clone for Delay<CLOCK_HZ> {
    fn clone(&self) -> Self {
        Self { _no_send: PhantomData }
    }
}

const SYST_CSR_ENABLE: u32 = 1 << 0;
const SYST_CSR_CLKSOURCE: u32 = 1 << 2;
const SYST_CSR_COUNTFLAG: u32 = 1 << 16;

impl<const CLOCK_HZ: u32> Delay<CLOCK_HZ> {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn new(_syst: SYST) -> Self {
        Delay { _no_send: PhantomData }
    }

    pub fn delay_ticks(&mut self, ticks: u32) {
        // SAFETY: SYST was passed to `new`, so exclusive access to the systick
        // is shared by all clones of `Delay`. We fully reset its state here.
        // Delay does not impl send, so can't be used in an ISR.
        unsafe {
            let systick = &*SYST::PTR;
            systick.rvr.write(ticks);
            systick.cvr.write(0);
            systick.csr.write(SYST_CSR_ENABLE | SYST_CSR_CLKSOURCE);
            while systick.csr.read() & SYST_CSR_COUNTFLAG == 0 {}
            systick.csr.write(0);
        }
    }

    #[inline(always)]
    fn delay_us(&mut self, us: u32) {
        // The SysTick Reload Value register supports values between 1 and 0x00FFFFFF.
        const MAX_RVR: u32 = 0x00FF_FFFF;

        let mut ticks = us * (CLOCK_HZ / 1_000_000);

        while ticks != 0 {
            let current_ticks = if ticks <= MAX_RVR {
                ticks
            } else {
                MAX_RVR
            };

            self.delay_ticks(current_ticks);
            ticks -= current_ticks;
        }
    }
}

impl<const CLOCK_HZ: u32> DelayMs<u32> for Delay<CLOCK_HZ> {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms * 1_000);
    }
}

impl<const CLOCK_HZ: u32> DelayMs<u16> for Delay<CLOCK_HZ> {
    fn delay_ms(&mut self, ms: u16) {
        self.delay_ms(ms as u32);
    }
}

impl<const CLOCK_HZ: u32> DelayMs<u8> for Delay<CLOCK_HZ> {
    fn delay_ms(&mut self, ms: u8) {
        self.delay_ms(ms as u32);
    }
}

impl<const CLOCK_HZ: u32> DelayUs<u32> for Delay<CLOCK_HZ> {
    fn delay_us(&mut self, us: u32) {
        Delay::delay_us(self, us);
    }
}

impl<const CLOCK_HZ: u32> DelayUs<u16> for Delay<CLOCK_HZ> {
    fn delay_us(&mut self, us: u16) {
        Delay::delay_us(self, us as u32)
    }
}

impl<const CLOCK_HZ: u32> DelayUs<u8> for Delay<CLOCK_HZ> {
    fn delay_us(&mut self, us: u8) {
        Delay::delay_us(self, us as u32)
    }
}
