use core::marker::PhantomData;

use zeptos::samd::gpio::{TypePin, AlternateFunc};
use zeptos::samd::sercom::{Sercom, StaticSercom, I2cController, I2cError};
use defmt::{debug, info, Format};

use viking_protocol::protocol::i2c;
use viking_protocol::errors::{ERR_INVALID_COMMAND, ERR_MISSING_ARG, ERR_INVALID_STATE, ERR_ADDR_NACK, ERR_DATA_NACK, ERR_ARBITRATION_LOST};

use crate::const_bytes;
use crate::common::{Reader, Resource, ResourceMode, Writer, ErrorByte};

#[derive(Clone, Copy, Debug, PartialEq, Format)]
enum State {
    Idle,
    Read,
    ReadFirst,
    Write,
    AddrNack,
    ArbitrationLost,
}

pub struct SercomI2C<S: StaticSercom, const PIN_MUX: bool> {
    i2c: I2cController<S>,
    state: State,
}

impl<S: StaticSercom, const PIN_MUX: bool> ResourceMode for SercomI2C<S, PIN_MUX> {
    const PROTOCOL: u16 = i2c::controller::PROTOCOL;
    const DESCRIPTOR: &[u8] = {
        use i2c::controller::{ModeFlags, SpeedFlags};
        const_bytes!(
            i2c::controller::DescribeMode {
                flags: if PIN_MUX { ModeFlags::PINS } else { ModeFlags::EMPTY }
                    .union(ModeFlags::CLOCK_STRETCH)
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
        let sercom = unsafe { S::steal() };
        let i2c = I2cController::new(sercom);
        Ok(SercomI2C { i2c, state: State::Idle })
    }

    fn deinit(self, _resource: Resource) {
        info!("i2c deinit");
    }

    async fn command(&mut self, _resource: Resource, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<u8, ErrorByte> {
        use i2c::controller::cmd;

        match command {
            cmd::START => {
                let addr = req.take_first().ok_or(ERR_MISSING_ARG)?;
                debug!("i2c start {:x}", addr);

                match self.i2c.start(addr).await {
                    Ok(()) => {
                        debug!("i2c start -> ack");
                        self.state = if addr & 0x01 != 0 { State::ReadFirst } else { State::Write };
                        Ok(0)
                    }
                    Err(I2cError::NoAcknowledge) => {
                        debug!("i2c addr nack");
                        self.state = State::AddrNack;
                        Ok(ERR_ADDR_NACK) // non-fatal
                    }
                    Err(I2cError::ArbitrationLost) => {
                        debug!("i2c addr arbitration lost");
                        self.state = State::ArbitrationLost;
                        Err(ERR_ARBITRATION_LOST)
                    }
                }
            }
            cmd::STOP => {
                debug!("i2c stop");
                self.i2c.stop();
                Ok(0)
            }
            cmd::READ => {
                let len = req.take_first().ok_or(ERR_MISSING_ARG)? as u8;
                for _ in 0..len {
                    let r = match self.state {
                        State::ReadFirst => {
                            self.state = State::Read;
                            Ok(self.i2c.read_first())
                        }
                        State::Read => self.i2c.read_next().await,
                        State::AddrNack => {
                            return Err(ERR_ADDR_NACK);
                        }
                        State::ArbitrationLost => {
                            return Err(ERR_ARBITRATION_LOST);
                        }
                        _ => return Err(ERR_INVALID_STATE),
                    };

                    match r {
                        Ok(byte) => {
                            debug!("i2c read byte -> {:02x}", byte);
                            res.put(byte)?;
                        }
                        Err(I2cError::NoAcknowledge) => {
                            debug!("i2c data nack");
                            return Err(ERR_DATA_NACK);
                        }
                        Err(I2cError::ArbitrationLost) => {
                            debug!("i2c read arbitration lost");
                            self.state = State::ArbitrationLost;
                            return Err(ERR_ARBITRATION_LOST);
                        }
                    }
                }
                Ok(0)
            }
            cmd::WRITE => {
                let buf = req.take_len().ok_or(ERR_MISSING_ARG)?;

                match self.state {
                    State::Write => (),
                    State::AddrNack => {
                        return Err(ERR_ADDR_NACK);
                    }
                    State::ArbitrationLost => {
                        return Err(ERR_ARBITRATION_LOST);
                    }
                    _ => return Err(ERR_INVALID_STATE),
                };

                for &b in buf {
                    match self.i2c.write(b).await {
                        Ok(()) => {
                            debug!("i2c write byte {:02x} -> ack", b);
                        }
                        Err(I2cError::NoAcknowledge) => {
                            debug!("i2c write byte {:02x} -> nack", b);
                            return Err(ERR_DATA_NACK);
                        }
                        Err(I2cError::ArbitrationLost) => {
                            debug!("i2c write byte {:02x} -> arbitration lost", b);
                            self.state = State::ArbitrationLost;
                            return Err(ERR_ARBITRATION_LOST);
                        }
                    }
                }

                Ok(0)
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
