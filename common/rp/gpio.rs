use core::marker::PhantomData;

use zeptos::{rp::gpio::*, Runtime};
use defmt::info;
use viking_protocol::protocol::{gpio, led};

use crate::common::{Writer, Reader, ResourceMode};

pub struct Gpio<P>(PhantomData<P>);

impl<P: TypePin> ResourceMode for Gpio<P> {
    const PROTOCOL: u16 = gpio::pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
        info!("gpio{} init", P::DYN.pin);
        P::set_function(Function::F5);
        Ok(Gpio(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        info!("gpio{} deinit", P::DYN.pin);
        P::oe_clr();
        P::disable();
    }

    async fn command(&self, _resource: Resource, command: u8, _buf: &mut Reader<'_>, response: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::gpio::pin::cmd;

        match command {
            cmd::FLOAT => {
                P::oe_clr();
                Ok(())
            }
            cmd::READ => {
                let byte: u8 = if P::read() { 0x01 } else { 0x00 };
                response.put(byte)?;
                Ok(())
            }
            cmd::LOW => {
                P::out_clr();
                P::oe_set();
                Ok(())
            }
            cmd::HIGH => {
                P::out_set();
                P::oe_set();
                Ok(())
            }
            _ => Err(())
        }
    }
}

/*
pub struct LevelInterrupt<P, const CH: u8>{
    _p: PhantomData<P>,
    event: Cell<Option<bool>>,
}

impl<P: TypePin, const CH: u8> ResourceMode for LevelInterrupt<P, CH> {
    const PROTOCOL: u16 = gpio::level_interrupt::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_config: &[u8]) -> Result<Self, ()> {
        info!("level_interrupt init {}", P::DYN.pin);
        P::set_function(Function::F5);
        Ok(LevelInterrupt { _p: PhantomData, event: Cell::new(None) })
    }

    fn deinit(self) {
        info!("level_interrupt deinit");
        P::disable();
    }

    async fn command(&self, command: u8, _buf: &mut &[u8], _response: &mut Writer<'_>) -> Result<(), ()> {
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
            //EVENT_CHANGE.notify();
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
                unsafe { INT.get_unchecked() }.subscribe(waker);
            }
        }
    }
}

type Sense = zeptos::rp::pac::io::Int;

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
    scopeguard::defer! {
        eic.intenclr.write(|w| unsafe { w.bits(1 << ch) });
    };
    unsafe { INT.get_unchecked() }.until(|| {
        eic.intflag.read().bits() & (1<<ch) != 0
    }).await;
}

static INT: TaskOnly<Interrupt> = unsafe { TaskOnly::new(Interrupt::new()) };

#[interrupt]
fn IO_IRQ_BANK0() {
    unsafe { INT.get_unchecked().notify(); }
}

*/

pub struct Led<P, const ACTIVE: bool, const COLOR: u8>(PhantomData<P>);

impl<P: TypePin, const ACTIVE: bool, const COLOR: u8> ResourceMode for Led<P, {ACTIVE}, {COLOR}> {
    const PROTOCOL: u16 = led::binary::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[COLOR];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
        P::set_function(Function::F5);
        if ACTIVE {
            P::out_set();
        } else {
            P::out_clr();
        }
        P::oe_set();
        Ok(Led(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::oe_clr();
        P::disable();
    }

    async fn command(&self, _resource: Resource, command: u8, _req: &mut Reader<'_>, _res: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::led::binary::cmd;

        match command {
            cmd::OFF => {
                if ACTIVE {
                    P::out_clr();
                } else {
                    P::out_set();
                }
                Ok(())
            }
            cmd::ON => {
                if ACTIVE {
                    P::out_set();
                } else {
                    P::out_clr();
                }
                Ok(())
            }
            _ => Err(())
        }
    }
}
