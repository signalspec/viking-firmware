use core::{cell::Cell, marker::PhantomData, task::Waker};

use super::pin::{PinId, IoPin};
use defmt::{debug, info};
use lilos::exec::Notify;
use viking_protocol::protocol::gpio;
use viking_protocol::AsBytes;
use atsamd_hal::pac::{eic::config, interrupt, EIC};

use crate::{viking::{const_bytes, ResourceMode, Writer}, viking_sam0::pin::Alternate, EVENT_CHANGE};

pub struct Gpio<P>(PhantomData<P>);

impl<P: PinId> ResourceMode for Gpio<P> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            gpio::pin::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("gpio init {:?} {:?}", P::DYN.group as u8, P::DYN.num);
        IoPin::<P>::pincfg().write(|w| w.inen().set_bit());
        IoPin::<P>::enable_sampling();
        Ok(Gpio(PhantomData))
    }

    fn deinit(self) {
        info!("gpio deinit");
        IoPin::<P>::dirclr();
    }

    async fn command(&self, command: u8, buf: &mut &[u8], response: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::gpio::pin::cmd;
        
        match command {
            cmd::FLOAT => {
                IoPin::<P>::dirclr();
                Ok(())
            }
            cmd::READ => {
                let byte: u8 = if IoPin::<P>::read() { 0x01 } else { 0x00 };
                response.put(byte)?;
                Ok(())
            }
            cmd::LOW => {
                IoPin::<P>::outclr();
                IoPin::<P>::dirset();
                Ok(())
            }
            cmd::HIGH => {
                IoPin::<P>::outset();
                IoPin::<P>::dirset();
                Ok(())
            }
            _ => Err(())
        }
    }
}


pub struct LevelInterrupt<P, const CH: u8>{
    _p: PhantomData<P>,
    event: Cell<Option<bool>>,
}

impl<P: PinId, const CH: u8> ResourceMode for LevelInterrupt<P, CH> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            gpio::level_interrupt::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("level_interrupt init {:?} {:?}", P::DYN.group as u8, P::DYN.num);
        IoPin::<P>::alternate(Alternate::A);
        Ok(LevelInterrupt { _p: PhantomData, event: Cell::new(None) })
    }

    fn deinit(self) {
        info!("level_interrupt deinit");
        IoPin::<P>::reset();
    }

    async fn command(&self, command: u8, buf: &mut &[u8], response: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::gpio::level_interrupt::cmd;
        
        let (sense, event) = match command {
            cmd::WAIT_LOW => (Sense::LOW, None),
            cmd::WAIT_HIGH => (Sense::HIGH, None),
            cmd::EVT_LOW => (Sense::LOW, Some(false)),
            cmd::EVT_HIGH => (Sense::HIGH, Some(true)),
            _ => return Err(())
        };

        configure_interrupt(CH, sense);
        self.event.set(event);

        if event.is_none() {
            wait_interrupt(CH).await;
        } else {
            EVENT_CHANGE.notify();
        }

        Ok(())
    }

    fn poll_event(&self, waker: &Waker, resource: u8, buf: &mut Writer<'_>) {
        if let Some(level) = self.event.get() {
            let eic = unsafe { EIC::steal() };
            if eic.intflag.read().bits() & (1<<CH) != 0 {
                if buf.put(resource | ((level as u8) << 6)).is_ok() {
                    self.event.set(None);
                }
            } else {
                IRQ.subscribe(waker);
            }
        }
    }
}

type Sense = atsamd_hal::pac::eic::config::SENSE0SELECT_A;

fn configure_interrupt(ch: u8, sense: Sense) {
    let eic = unsafe { EIC::steal() };
    eic.config[if ch > 7 { 1 } else { 0 }].modify(|_, w| {
        unsafe {
            match ch & 0b111 {
                0 => w.sense0().bits(sense as u8),
                1 => w.sense1().bits(sense as u8),
                2 => w.sense2().bits(sense as u8),
                3 => w.sense3().bits(sense as u8),
                4 => w.sense4().bits(sense as u8),
                5 => w.sense5().bits(sense as u8),
                6 => w.sense6().bits(sense as u8),
                7 => w.sense7().bits(sense as u8),
                _ => unreachable!(),
            }
        }
    });
    eic.intflag.write(|w| unsafe { w.bits(1 << ch) });
    eic.intenset.write(|w| unsafe { w.bits(1 << ch) });
}

async fn wait_interrupt(ch: u8) {
    let eic = unsafe { EIC::steal() };
    IRQ.until(|| {
        eic.intflag.read().bits() & (1<<ch) != 0
    }).await;
    eic.intenclr.write(|w| unsafe { w.bits(1 << ch) });
}

static IRQ: Notify = Notify::new();

#[interrupt]
fn EIC() {
    let eic = unsafe { EIC::steal() };
    eic.intenclr.write(|w| unsafe { w.bits(eic.intflag.read().bits()) });
    IRQ.notify();
}
