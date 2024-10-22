//! Delays

use core::future::Future;

use zeptos::cortex_m::SysTick;

pub trait AsyncDelayUs {
    const MAX: u32;

    async fn delay_us(&mut self, us: u32);
}

impl AsyncDelayUs for SysTick {
    const MAX: u32 = 0x00FF_FFFF / (zeptos::CLOCK_HZ / 1_000_000);

    fn delay_us(&mut self, us: u32) -> impl Future<Output = ()> {
        SysTick::delay_us(self, us)
    }
}
