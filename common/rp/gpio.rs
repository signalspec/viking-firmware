use core::{cell::Cell, marker::PhantomData};

use zeptos::{Runtime, TaskOnly, rp::gpio::*, task};
use defmt::{debug, info};
use viking_protocol::protocol::{gpio, led};

use crate::common::{Writer, Reader, ResourceMode, Resource, ErrorByte};

pub fn init(rt: Runtime) {
    gpio_interrupt_task(rt).spawn(rt);
}

pub struct Gpio<P>(PhantomData<P>);

impl<P: TypePin> ResourceMode for Gpio<P> {
    const PROTOCOL: u16 = gpio::pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("gpio{} init", P::DYN.pin);
        P::set_function(Function::F5);
        Ok(Gpio(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        info!("gpio{} deinit", P::DYN.pin);
        P::oe_clr();
        P::disable();
    }

    async fn command(&mut self, _resource: Resource, command: u8, _buf: &mut Reader<'_>, response: &mut Writer<'_>) -> Result<(), ()> {
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

pub struct LevelInterrupt<P>{
    _p: PhantomData<P>,
}

impl<P: TypePin> ResourceMode for LevelInterrupt<P> {
    const PROTOCOL: u16 = gpio::level_interrupt::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("level_interrupt init {}", P::DYN.pin);
        P::set_function(Function::F5);
        Ok(LevelInterrupt { _p: PhantomData })
    }

    fn deinit(self, resource: Resource) {
        info!("level_interrupt deinit");
        GPIO_INT_STATE.get(resource.rt)[P::DYN.pin as usize].set(EventWatch::None);
        P::disable();
    }

    async fn command(&mut self, resource: Resource, command: u8, _buf: &mut Reader<'_>, _response: &mut Writer<'_>) -> Result<(), ()> {
        let rt = resource.rt();
        use viking_protocol::protocol::gpio::level_interrupt::cmd;

        match command {
            cmd::WAIT_LOW => {
                P::DYN.wait_level(rt, false).await;
            }
            cmd::WAIT_HIGH => {
                P::DYN.wait_level(rt, true).await;
            }
            cmd::EVT_LOW => {
                GPIO_INT_STATE.get(rt)[P::DYN.pin as usize].set(EventWatch::LevelLow(resource));
                gpio_interrupt_task(rt).wake();
             }
            cmd::EVT_HIGH => {
                GPIO_INT_STATE.get(rt)[P::DYN.pin as usize].set(EventWatch::LevelHigh(resource));
                gpio_interrupt_task(rt).wake();
            }
            _ => return Err(())
        };

        Ok(())
    }
}

static GPIO_INT_STATE: TaskOnly<[Cell<EventWatch>; 30]> = unsafe { TaskOnly::new_unsend([const { Cell::new(EventWatch::None) }; 30]) };

#[derive(Copy, Clone)]
enum EventWatch {
    None,
    LevelLow(Resource),
    LevelHigh(Resource),
}

#[task]
async fn gpio_interrupt_task(rt: Runtime) {
    zeptos::rp::gpio::BANK0_INT.get_pinned(rt).until(|| {
        for (i, state) in GPIO_INT_STATE.get(rt).iter().enumerate() {
            let pin = zeptos::rp::gpio::IoPin::bank0(i as u8);
            match state.get() {
                EventWatch::None => {},
                EventWatch::LevelLow(r) => {
                    let status = pin.interrupt_status();
                    debug!("gpio{} polling for level low: {:04b}", i, status.bits());
                    if status.contains(EventMask::LOW) {
                        r.send_event(gpio::level_interrupt::evt::LOW);
                        state.set(EventWatch::None);
                    } else {
                        pin.enable_interrupts(EventMask::LOW);
                    }
                }
                EventWatch::LevelHigh(r) => {
                    let status = pin.interrupt_status();
                    debug!("gpio{} polling for level high: {:04b}", i, status.bits());
                    if status.contains(EventMask::HIGH) {
                        r.send_event(gpio::level_interrupt::evt::HIGH);
                        state.set(EventWatch::None);
                    } else {
                        pin.enable_interrupts(EventMask::HIGH);
                    }
                },
            }
        }

        false
    }).await;
}

pub struct Led<P, const ACTIVE: bool, const COLOR: u8>(PhantomData<P>);

impl<P: TypePin, const ACTIVE: bool, const COLOR: u8> ResourceMode for Led<P, {ACTIVE}, {COLOR}> {
    const PROTOCOL: u16 = led::binary::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[COLOR];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
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

    async fn command(&mut self, _resource: Resource, command: u8, _req: &mut Reader<'_>, _res: &mut Writer<'_>) -> Result<(), ()> {
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
