use core::{cell::Cell, marker::PhantomData};

use zeptos::samd::{gpio::TypePin, pac::sercom0::I2CM};
use zeptos::samd::gpio:: AlternateFunc ;
use defmt::{debug, info, Format};

use viking_protocol::protocol::i2c;
use zeptos::Runtime;

use crate::const_bytes;
use crate::common::{Reader, ResourceMode, Writer};
use super::sercom::{ Sercom, DynSercom };

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
    const PROTOCOL: u16 = i2c::controller::PROTOCOL;
    const DESCRIPTOR: &[u8] = {
        use i2c::controller::{ModeFlags, SpeedFlags};
        const_bytes!(
            i2c::controller::DescribeMode {
                flags: ModeFlags::CLOCK_STRETCH
                    .union(ModeFlags::BYTE_AT_A_TIME)
                    .union(ModeFlags::WRITE_THEN_READ)
                    .union(ModeFlags::REPEATED_START)
                    .union(ModeFlags::REPEATED_START_SAME_ADDRESS)
                    .union(ModeFlags::ZERO_LEN_WRITE),
                speed: SpeedFlags::STANDARD,
            }
        )
    };

    fn init(_config: &[u8]) -> Result<Self, ()> {
        info!("i2c init");
        init(DynSercom(S::NUM));
        Ok(SercomI2C { _p: PhantomData, state: Cell::new(State::Idle) })
    }

    fn deinit(self) {
        info!("i2c deinit");
        deinit(DynSercom(S::NUM));
    }

    async fn command(&self, _rt: Runtime, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<(), ()> {
        use i2c::controller::cmd;
        let sercom = DynSercom(S::NUM);

        match command {
            cmd::START => {
                let addr = req.take_first().ok_or(())?;
                debug!("i2c start {:x} {:?}", addr, self.state.get());
                let r = start(sercom, addr, &self.state).await;
                debug!("i2c start -> {} {:?}", r, self.state.get());
                res.put(r)?;
                Ok(())
            }
            cmd::STOP => {
                debug!("i2c stop {:?}", self.state.get());
                stop(sercom, &self.state).await;
                Ok(())
            }
            cmd::READ => {
                debug!("i2c read {:?}", self.state.get());
                let len = req.take_first().ok_or(())? as u8;
                read(sercom, len, res, &self.state).await
            }
            cmd::WRITE => {
                debug!("i2c write {:?}", self.state.get());
                let buf = req.take_len().ok_or(())?;
                write(sercom, buf, &self.state).await?;
                Ok(())
            }
            _ => Err(())
        }
    }
}

pub struct SercomSCLPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S: Sercom, M: AlternateFunc> ResourceMode for SercomSCLPin<P, S, M> {
    const PROTOCOL: u16 = i2c::scl::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_config: &[u8]) -> Result<Self, ()> {
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
    const PROTOCOL: u16 = i2c::sda::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_config: &[u8]) -> Result<Self, ()> {
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

    for _ in 0..n {
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
