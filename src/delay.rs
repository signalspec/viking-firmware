//! Delays

use zeptos::cortex_m::SysTick;

pub trait AsyncDelayUs {
    const MAX: u32;

    async fn delay_us(&mut self, us: u32);
}

impl AsyncDelayUs for SysTick {
    const MAX: u32 = 0x00FF_FFFF / (zeptos::CLOCK_HZ / 1_000_000);

    async fn delay_us(&mut self, us: u32) {
        self.delay_us(us).await
    }
}
