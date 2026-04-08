use core::{cell::Cell, marker::PhantomData};

use zeptos::samd::{gpio::TypePin, pac::sercom0::I2CM};
use zeptos::samd::gpio:: AlternateFunc ;
use defmt::{debug, info, Format};

use viking_protocol::protocol::i2c;
use viking_protocol::errors::{ERR_INVALID_COMMAND, ERR_MISSING_ARG, ERR_INVALID_STATE};

use crate::const_bytes;
use crate::common::{Reader, Resource, ResourceMode, Writer, ErrorByte};
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
    state: State,
}

impl<S: Sercom> ResourceMode for SercomI2C<S> {
    const PROTOCOL: u16 = i2c::controller::PROTOCOL;
    const DESCRIPTOR: &[u8] = {
        use i2c::controller::{ModeFlags, SpeedFlags};
        const_bytes!(
            i2c::controller::DescribeMode {
                flags: ModeFlags::CLOCK_STRETCH
                    .union(ModeFlags::SPLIT)
                    .union(ModeFlags::WRITE_THEN_READ)
                    .union(ModeFlags::REPEATED_START)
                    .union(ModeFlags::REPEATED_START_SAME_ADDRESS)
                    .union(ModeFlags::ZERO_LEN_WRITE)
                    .union(ModeFlags::ADDR_NACK)
                    .union(ModeFlags::PRECISE_NACK),
                speed: SpeedFlags::STANDARD,
            }
        )
    };

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("i2c init");
        init(DynSercom(S::NUM));
        Ok(SercomI2C { _p: PhantomData, state: State::Idle })
    }

    fn deinit(self, _resource: Resource) {
        info!("i2c deinit");
        deinit(DynSercom(S::NUM));
    }

    async fn command(&mut self, _resource: Resource, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<(), ErrorByte> {
        use i2c::controller::cmd;
        let sercom = DynSercom(S::NUM);

        match command {
            cmd::START => {
                let addr = req.take_first().ok_or(ERR_MISSING_ARG)?;
                debug!("i2c start {:x} {:?}", addr, self.state);
                let r = start(sercom, addr, &mut self.state).await;
                debug!("i2c start -> {} {:?}", r, self.state);
                res.put(r)?;
                Ok(())
            }
            cmd::STOP => {
                debug!("i2c stop {:?}", self.state);
                stop(sercom, &mut self.state).await;
                Ok(())
            }
            cmd::READ => {
                debug!("i2c read {:?}", self.state);
                let len = req.take_first().ok_or(ERR_MISSING_ARG)? as u8;
                read(sercom, len, res, &mut self.state).await
            }
            cmd::WRITE => {
                debug!("i2c write {:?}", self.state);
                let buf = req.take_len().ok_or(ERR_MISSING_ARG)?;
                let written = write(sercom, buf, &mut self.state).await?;
                res.put(written)?;
                Ok(())
            }
            _ => Err(ERR_INVALID_COMMAND)
        }
    }
}

pub struct SercomSCLPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S: Sercom, M: AlternateFunc> ResourceMode for SercomSCLPin<P, S, M> {
    const PROTOCOL: u16 = i2c::scl::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("sercom SCL init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::set_io();
    }
}

pub struct SercomSDAPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S, M: AlternateFunc> ResourceMode for SercomSDAPin<P, S, M> {
    const PROTOCOL: u16 = i2c::sda::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("sercom SDA init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
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

async fn start(sercom: DynSercom, addr: u8, state: &mut State) -> u8 {
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
        *state = State::Nack;
        1
    } else if addr & 0x01 != 0 {
        *state = State::ReadFirst;
        0
    } else {
        *state = State::Write;
        0
    }
}

async fn write(sercom: DynSercom, data: &[u8], state: &mut State) -> Result<u8, ErrorByte> {
    let regs = sercom.regs().i2cm();

    let mut sent = 0;

    for &b in data {
        if *state != State::Write {
            return Err(ERR_INVALID_STATE)
        }

        regs.data.write(|w| w.data().variant(b));
        sync_sysop(regs);

        regs.intenset.write(|w| { w.mb().set_bit() });

        sercom.notify().until(|| {
            regs.intflag.read().mb().bit_is_set()
        }).await;

        if check_error(regs).is_err() {
            *state = State::Nack;
            break;
        }

        sent += 1;
    }

    Ok(sent)
}

async fn read(sercom: DynSercom, n: u8, writer: &mut Writer<'_>, state: &mut State) -> Result<(), ErrorByte> {
    let regs = sercom.regs().i2cm();

    for _ in 0..n {
        if *state == State::Read {
            // Ack previous byte, read the next
            regs.ctrlb.write(|w| w.cmd().variant(0x02));
            sync_sysop(regs);
            regs.intenset.write(|w| { w.sb().set_bit() });
            sercom.notify().until(|| {
                regs.intflag.read().sb().bit_is_set()
            }).await;
        } else if *state == State::ReadFirst {
            // First byte has already been read
            *state = State::Read;
        } else {
            return Err(ERR_INVALID_STATE)
        }

        writer.put(regs.data.read().data().bits())?;

        if check_error(regs).is_err() {
            *state = State::Nack;
        }
    }

    Ok(())
}

async fn stop(sercom: DynSercom, state: &mut State) {
    let regs = sercom.regs().i2cm();

    regs.ctrlb.write(|w| {
        w.ackact().set_bit(); // send nack if read
        w.cmd().variant(0x3)
    });
    sync_sysop(regs);

    *state = State::Idle;
}
