use core::{cell::Cell, marker::PhantomData, task::Waker};

use zeptos::{Runtime, samd::gpio::{Alternate, TypePin}, Interrupt, TaskOnly};
use defmt::info;
use viking_protocol::protocol::{gpio, led};
use zeptos::samd::pac::{interrupt, EIC};

use crate::common::{Reader, Resource, ResourceMode, Writer};

pub struct Gpio<P>(PhantomData<P>);

impl<P: TypePin> ResourceMode for Gpio<P> {
    const PROTOCOL: u16 = gpio::pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
        info!("gpio init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::pincfg().write(|w| w.inen().set_bit());
        P::enable_sampling();
        Ok(Gpio(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        info!("gpio deinit");
        P::dirclr();
    }

    async fn command(&self, _resource: Resource, command: u8, _req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::gpio::pin::cmd;

        match command {
            cmd::FLOAT => {
                P::dirclr();
                Ok(())
            }
            cmd::READ => {
                let byte: u8 = if P::read() { 0x01 } else { 0x00 };
                res.put(byte)?;
                Ok(())
            }
            cmd::LOW => {
                P::outclr();
                P::dirset();
                Ok(())
            }
            cmd::HIGH => {
                P::outset();
                P::dirset();
                Ok(())
            }
            _ => Err(())
        }
    }
}


pub struct LevelInterrupt<P, const CH: u8>{
    _p: PhantomData<P>,
}

impl<P: TypePin, const CH: u8> ResourceMode for LevelInterrupt<P, CH> {
    const PROTOCOL: u16 = gpio::level_interrupt::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(resource: Resource, _config: &[u8]) -> Result<Self, ()> {
        let rt = resource.rt;
        info!("level_interrupt init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(Alternate::A);
        let state = &GPIO_INT_STATE.get(rt)[CH as usize];

        if !matches!(state.get(), EventWatch::Free) {
            return Err(());
        }

        state.set(EventWatch::Idle);
        Ok(LevelInterrupt { _p: PhantomData })
    }

    fn deinit(self, resource: Resource) {
        let rt = resource.rt;
        info!("level_interrupt deinit");
        GPIO_INT_STATE.get(rt)[CH as usize].set(EventWatch::Free);
        P::set_io();
    }

    async fn command(&self, resource: Resource, command: u8, _req: &mut Reader<'_>, _res: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::gpio::level_interrupt::cmd;
        let rt = resource.rt;

        let (sense, event) = match command {
            cmd::WAIT_LOW => (Sense::LOW, None),
            cmd::WAIT_HIGH => (Sense::HIGH, None),
            cmd::EVT_LOW => (Sense::LOW, Some(false)),
            cmd::EVT_HIGH => (Sense::HIGH, Some(true)),
            _ => return Err(())
        };

        configure_interrupt(CH, sense);

        if let Some(event) = event {
            GPIO_INT_STATE.get(rt)[CH as usize].set(EventWatch::Level(event, resource));
            enable_interrupt(CH);
        } else {
            wait_interrupt(rt, CH).await;
        }

        Ok(())
    }
}

type Sense = zeptos::samd::pac::eic::config::SENSE0SELECT_A;

static GPIO_INT_STATE: TaskOnly<[Cell<EventWatch>; 16]> = unsafe { TaskOnly::new_unsend([const { Cell::new(EventWatch::Free) }; 16]) };

#[derive(Clone, Copy)]
enum EventWatch {
    Free,
    Idle,
    Level(bool, Resource),
}

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
}

fn enable_interrupt(ch: u8) {
    let eic = unsafe { EIC::steal() };
    eic.intflag.write(|w| unsafe { w.bits(1 << ch) });
    eic.intenset.write(|w| unsafe { w.bits(1 << ch) });
}

async fn wait_interrupt(rt: Runtime, ch: u8) {
    let eic = unsafe { EIC::steal() };
    INT.get(rt).until(|| {
        if eic.intflag.read().bits() & (1<<ch) == 0 {
            enable_interrupt(ch);
            false
        } else {
            true
        }
    }).await;
}

static INT: TaskOnly<Interrupt> = TaskOnly::new(Interrupt::new());

#[interrupt]
fn EIC() {
    use viking_protocol::protocol::gpio::level_interrupt::evt;
    let eic = unsafe { EIC::steal() };
    eic.intenclr.write(|w| unsafe { w.bits(0xff) });

    let flags = eic.intflag.read().bits();
    let states = unsafe { GPIO_INT_STATE.get_unchecked() };
    for (ch, state) in states.iter().enumerate() {
        match state.get() {
            EventWatch::Free | EventWatch::Idle => {}
            EventWatch::Level(level, resource) => {
                defmt::debug!("Polling GPIO IRQ {} for level {} in {:016b}", ch, level, flags);
                if flags & (1<<ch) != 0 {
                    resource.send_event(if level { evt::HIGH } else { evt::LOW });
                    state.set(EventWatch::Idle);
                } else {
                    enable_interrupt(ch as u8);
                }
            }
        }

    }

    unsafe { INT.get_unchecked().notify(); }
}

pub struct Led<P, const ACTIVE: bool, const COLOR: u8>(PhantomData<P>);

impl<P: TypePin, const ACTIVE: bool, const COLOR: u8> ResourceMode for Led<P, {ACTIVE}, {COLOR}> {
    const PROTOCOL: u16 = led::binary::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[COLOR];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
        info!("led init {:?} {:?}", P::DYN.group, P::DYN.pin);
        if ACTIVE {
            P::outset();
        } else {
            P::outclr();
        }
        P::dirset();
        Ok(Led(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::dirclr();
        info!("led deinit");
    }

    async fn command(&self, _resource: Resource, command: u8, _req: &mut Reader<'_>, _res: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::led::binary::cmd;

        match command {
            cmd::OFF => {
                if ACTIVE {
                    P::outclr();
                } else {
                    P::outset();
                }
                Ok(())
            }
            cmd::ON => {
                if ACTIVE {
                    P::outset();
                } else {
                    P::outclr();
                }
                Ok(())
            }
            _ => Err(())
        }
    }
}
