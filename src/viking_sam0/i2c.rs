use core::{cell::Cell, marker::PhantomData};

use zeptos::samd::{gpio::TypePin, pac::{interrupt, sercom0::{self, RegisterBlock, I2CM}, Interrupt, SERCOM0, SERCOM1}};
use zeptos::samd::gpio::{ IoPin, AlternateFunc };
use defmt::{debug, info, Format};

use viking_protocol::protocol::i2c;
use viking_protocol::AsBytes;

use crate::{viking::{const_bytes, take_first, take_len, ResourceMode, Writer}, viking_sam0::sercom::{ Sercom, DynSercom }};

#[derive(Clone, Copy, Debug, PartialEq, Format)]
enum State {
    Idle,
    Read,
    ReadFirst,
    Write,
    Nack,
}
pub struct SercomI2C<S> {
    _p: PhantomData<S>,
    state: Cell<State>,
}

impl<S: Sercom> ResourceMode for SercomI2C<S> {
    fn describe() -> &'static [u8] {
        use i2c::controller::{ModeFlags, SpeedFlags};
        const_bytes!(
            i2c::controller::DescribeMode {
                protocol: viking_protocol::ConstU16::new(),
                flags: ModeFlags::CLOCK_STRETCH
                    .union(ModeFlags::BYTE_AT_A_TIME)
                    .union(ModeFlags::WRITE_THEN_READ)
                    .union(ModeFlags::REPEATED_START)
                    .union(ModeFlags::REPEATED_START_SAME_ADDRESS)
                    .union(ModeFlags::ZERO_LEN_WRITE),
                speed: SpeedFlags::SPEED_STANDARD,
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("i2c init");
        init(DynSercom(S::NUM));
        Ok(SercomI2C { _p: PhantomData, state: Cell::new(State::Idle) })
    }

    fn deinit(self) {
        info!("i2c deinit");
        deinit(DynSercom(S::NUM));
    }

    async fn command(&self, command: u8, buf: &mut &[u8], response: &mut Writer<'_>) -> Result<(), ()> {
        use i2c::controller::cmd;
        let sercom = DynSercom(S::NUM);
        
        match command {
            cmd::START => {
                let addr = take_first(buf).ok_or(())?;
                debug!("i2c start {:x} {:?}", addr, self.state.get());
                let res = start(sercom, addr, &self.state).await;
                debug!("i2c start -> {} {:?}", res, self.state.get());
                response.put(res);
                Ok(())
            }
            cmd::STOP => {
                debug!("i2c stop {:?}", self.state.get());
                stop(sercom, &self.state).await;
                Ok(())
            }
            cmd::READ => {
                debug!("i2c read {:?}", self.state.get());
                let len = take_first(buf).ok_or(())? as u8;
                read(sercom, len, response, &self.state).await
            }
            cmd::WRITE => {
                debug!("i2c write {:?}", self.state.get());
                let buf = take_len(buf).ok_or(())?;
                write(sercom, buf, &self.state).await?;
                Ok(())
            }
            _ => Err(())
        }
    }
}

pub struct SercomSCLPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S: Sercom, M: AlternateFunc> ResourceMode for SercomSCLPin<P, S, M> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            i2c::scl::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("sercom SCL init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self) {
        P::set_io();
    }
}

pub struct SercomSDAPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S, M: AlternateFunc> ResourceMode for SercomSDAPin<P, S, M> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            i2c::sda::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("sercom SDA init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self) {
        P::set_io();
    }
}

fn init(sercom: DynSercom) {
    let regs = sercom.regs().i2cm();
    regs.ctrla.write(|w| w.mode().i2c_master());
    regs.baud.write(|w| w.baud().variant(235) ); // 100kHz
    regs.ctrla.write(|w| 
        w.mode().i2c_master()
        .enable().set_bit()
    );

    while regs.syncbusy.read().enable().bit_is_set() {}
    regs.status.write(|w| w.busstate().variant(1) ); // set idle
    while regs.syncbusy.read().sysop().bit_is_set() {}
}

fn deinit(sercom: DynSercom) {
    sercom.regs().i2cm().ctrla.write(|w| w.swrst().set_bit());
}

fn check_error(regs: &I2CM) -> Result<(), ()> {
    let status = regs.status.read();
    if status.buserr().bit_is_set() {
        debug!("buserr");
        return Err(());
    }
    if status.arblost().bit_is_set() {
        debug!("arblost");
        return Err(());
    }
    if status.rxnack().bit_is_set() {
        debug!("rxnack");
        return Err(());
    }
    Ok(())
}

#[inline]
fn sync_sysop(regs: &I2CM) {
    while regs.syncbusy.read().sysop().bit_is_set() {}
}

async fn start(sercom: DynSercom, addr: u8, state: &Cell<State>) -> u8 {
    let regs = sercom.regs().i2cm();

    regs.addr.write(|w| w.addr().variant(addr as u16));
    regs.intenset.write(|w| {
        w.mb().set_bit();
        w.sb().set_bit();
        w.error().set_bit()
    });

    sync_sysop(regs);

    sercom.notify().until(|| {
        let flags = regs.intflag.read();
        flags.mb().bit_is_set() | flags.sb().bit_is_set() | flags.error().bit_is_set()
    }).await;

    if check_error(regs).is_err() {
        state.set(State::Nack);
        1
    } else if addr & 0x01 != 0 {
        state.set(State::ReadFirst);
        0
    } else {
        state.set(State::Write);
        0
    }
}

async fn write(sercom: DynSercom, data: &[u8], state: &Cell<State>) -> Result<(), ()> {
    let regs = sercom.regs().i2cm();

    for &b in data {
        if state.get() != State::Write {
            return Err(())
        }

        regs.data.write(|w| w.data().variant(b));
        sync_sysop(regs);
    
        regs.intenset.write(|w| { w.mb().set_bit() });
    
        sercom.notify().until(|| {
            regs.intflag.read().mb().bit_is_set()
        }).await;
    
        if check_error(regs).is_err() {
            state.set(State::Nack);
        }
    }

    Ok(())
}

async fn read(sercom: DynSercom, n: u8, writer: &mut Writer<'_>, state: &Cell<State>) -> Result<(), ()> {
    let regs = sercom.regs().i2cm();

    for i in 0..n {
        if state.get() == State::Read {
            // Ack previous byte, read the next
            regs.ctrlb.write(|w| w.cmd().variant(0x02));
            sync_sysop(regs);
            regs.intenset.write(|w| { w.sb().set_bit() });
            sercom.notify().until(|| {
                regs.intflag.read().sb().bit_is_set()
            }).await;
        } else if state.get() == State::ReadFirst {
            // First byte has already been read
            state.set(State::Read);
        } else {
            return Err(())
        }

        writer.put(regs.data.read().data().bits())?;
    
        if check_error(regs).is_err() {
            state.set(State::Nack);
        }
    }

    Ok(())
}

async fn stop(sercom: DynSercom, state: &Cell<State>) {
    let regs = sercom.regs().i2cm();

    regs.ctrlb.write(|w| {
        w.ackact().set_bit(); // send nack if read
        w.cmd().variant(0x3)
    });
    sync_sysop(regs);

    state.set(State::Idle)
}

