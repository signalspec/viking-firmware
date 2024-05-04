//! Delays
//! Based on atsamd-rs under MIT or Apache-2.0

use core::marker::PhantomData;

use cortex_m::peripheral::{syst, SYST};
use cortex_m_rt::exception;
use defmt::debug;
use lilos::exec::Notify;
use scopeguard::defer;

/// System timer (SysTick) as a delay provider
pub struct Delay<const CLOCK_HZ: u32> {
    _no_send: PhantomData<*const ()>,
}

const SYST_CSR_ENABLE: u32 = 1 << 0;
const SYST_CSR_TICKINT: u32 = 1 << 1;
const SYST_CSR_CLKSOURCE: u32 = 1 << 2;
const SYST_CSR_COUNTFLAG: u32 = 1 << 16;

impl<const CLOCK_HZ: u32> Delay<CLOCK_HZ> {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn new(_syst: SYST) -> Self {
        Delay { _no_send: PhantomData }
    }

    pub async fn delay_ticks(&mut self, ticks: u32) {
        unsafe {
            let systick = &*SYST::PTR;
            systick.rvr.write(ticks);
            systick.cvr.write(0);
            systick.csr.write(SYST_CSR_ENABLE | SYST_CSR_CLKSOURCE | SYST_CSR_TICKINT);
            defer! { systick.csr.write(0); }
            IRQ.until(|| {
                systick.csr.read() & SYST_CSR_COUNTFLAG != 0
            }).await;
        }
    }
}

static IRQ: Notify = Notify::new();

#[exception]
fn SysTick() {
    let syst = unsafe { &*SYST::PTR };
    unsafe { syst.csr.write(SYST_CSR_COUNTFLAG) }; // Clear ENABLE and TICKINT
    IRQ.notify()
}

pub trait AsyncDelayUs {
    const MAX: u32;

    async fn delay_us(&mut self, us: u32);
}

impl<const CLOCK_HZ: u32> AsyncDelayUs for Delay<CLOCK_HZ> {
    const MAX: u32 = 0x00FF_FFFF / (CLOCK_HZ / 1_000_000);

    async fn delay_us(&mut self, us: u32) {
        self.delay_ticks(us * (CLOCK_HZ / 1_000_000)).await
    }
}
