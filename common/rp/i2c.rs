use core::{marker::PhantomData};

use zeptos::rp::{gpio::{TypePin, Function}, i2c};
use defmt::{debug, info, Format};

use viking_protocol::protocol::i2c as i2c_proto;
use viking_protocol::errors::{ERR_MISSING_ARG, ERR_INVALID_STATE, ERR_ADDR_NACK, ERR_DATA_NACK, ERR_ARBITRATION_LOST, ERR_UNKNOWN, ERR_INVALID_COMMAND};

use crate::{common::ErrorByte, const_bytes};
use crate::common::{Reader, Resource, ResourceMode, Writer, req_from_bytes};

#[derive(Clone, Copy, Debug, PartialEq, Format)]
enum State {
    Idle,
    RestartRead,
    RestartWrite,
    Read,
    Write,
    AddrNack,
    DataNack,
    ArbitrationLost,
}

pub struct I2c<I: i2c::StaticInstance> {
    controller: i2c::Controller<I>,
    state: State,
}

impl<I: i2c::StaticInstance> ResourceMode for I2c<I> {
    const PROTOCOL: u16 = i2c_proto::controller::PROTOCOL;
    const DESCRIPTOR: &[u8] = {
        use i2c_proto::controller::{ModeFlags, SpeedFlags};
        const_bytes!(
            i2c_proto::controller::DescribeMode {
                flags: ModeFlags::PINS
                    .union(ModeFlags::CLOCK_STRETCH)
                    .union(ModeFlags::SPLIT)
                    .union(ModeFlags::WRITE_THEN_READ)
                    .union(ModeFlags::REPEATED_START_SAME_ADDRESS),
                speed: SpeedFlags::STANDARD.union(SpeedFlags::FAST).union(SpeedFlags::SLOW),
            }
        )
    };

    fn init(_resource: Resource, req: &[u8]) -> Result<Self, u8> {
        info!("i2c init");
        let req = req_from_bytes::<i2c_proto::controller::Config>(req);
        let mut config = i2c::Config::default();
        config.frequency = match req.speed {
            i2c_proto::controller::speed::SLOW => 10_000,
            i2c_proto::controller::speed::STANDARD => 100_000,
            i2c_proto::controller::speed::FAST => 400_000,
            _ => return Err(viking_protocol::errors::ERR_UNSUPPORTED_CLOCK),
        };
        let instance = unsafe { I::steal() };
        let controller = i2c::Controller::new(instance, config);
        Ok(I2c { controller, state: State::Idle })
    }

    fn deinit(self, _resource: Resource) {
        info!("i2c deinit");
    }

    async fn command(&mut self, _resource: Resource, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<u8, ErrorByte> {
        use i2c_proto::controller::cmd;

        match command {
            cmd::START => {
                let addr = req.take_first().ok_or(ERR_MISSING_ARG)?;
                debug!("i2c set address {:x} in state {:?}", addr, self.state);

                let read = addr & 1 != 0;
                let addr = (addr >> 1) as u16;
                match self.state {
                    State::Idle => {
                        self.controller.set_address(addr);
                        self.state = if read { State::Read } else { State::Write }
                    }
                    State::Read | State::Write => {
                        if addr != self.controller.get_address() {
                            defmt::warn!("i2c restart with different address {:x} != {:x}", addr, self.controller.get_address());
                            return Err(ERR_INVALID_STATE);
                        }
                        self.state = if read { State::RestartRead } else { State::RestartWrite };
                    }
                    _ => return Err(ERR_INVALID_STATE),
                }
                Ok(0)
            }
            cmd::STOP => {
                debug!("i2c stop in state {:?}", self.state);

                self.controller.abort().await;
                self.state = State::Idle;

                Ok(0)
            }
            cmd::READ => {
                debug!("i2c read in state {:?}", self.state);
                let len = req.take_first().ok_or(ERR_MISSING_ARG)?;

                let mut restart = match self.state{
                    State::Read => false,
                    State::RestartRead => {
                        self.state = State::Read;
                        true
                    }
                    State::AddrNack => return Err(ERR_ADDR_NACK),
                    State::DataNack => return Err(ERR_DATA_NACK),
                    State::ArbitrationLost => return Err(ERR_ARBITRATION_LOST),
                    _ => return Err(ERR_INVALID_STATE),
                };

                for _ in 0..len {
                    let data = self.controller.read(restart, false).await;
                    debug!("i2c read byte -> {:?}", data);
                    match data {
                        Ok(byte) => res.put(byte)?,
                        Err(i2c::Error::AddrNack) => {
                            self.state = State::AddrNack;
                            return Err(ERR_ADDR_NACK);
                        }
                        Err(i2c::Error::DataNack) => {
                            self.state = State::DataNack;
                            return Err(ERR_DATA_NACK);
                        }
                        Err(i2c::Error::ArbitrationLost) => {
                            self.state = State::ArbitrationLost;
                            return Err(ERR_ARBITRATION_LOST);
                        }
                        Err(_) => return Err(ERR_UNKNOWN),
                    }
                    restart = false;
                }

                Ok(0)
            }
            cmd::WRITE => {
                debug!("i2c write in state {:?}", self.state);
                let buf = req.take_len().ok_or(ERR_MISSING_ARG)?;

                let mut restart = match self.state {
                    State::Write => false,
                    State::RestartWrite => {
                        self.state = State::Write;
                        true
                    }
                    State::AddrNack => return Err(ERR_ADDR_NACK),
                    State::DataNack => return Err(ERR_DATA_NACK),
                    State::ArbitrationLost => return Err(ERR_ARBITRATION_LOST),
                    _ => return Err(ERR_INVALID_STATE),
                };

                for b in buf {
                    let res = self.controller.write(*b, restart, false).await;
                    debug!("i2c write byte {:02x} -> {:?}", b, res);
                    match res {
                        Ok(()) => (),
                        Err(i2c::Error::AddrNack) => {
                            self.state = State::AddrNack;
                            return Err(ERR_ADDR_NACK);
                        }
                        Err(i2c::Error::DataNack) => {
                            self.state = State::DataNack;
                            return Err(ERR_DATA_NACK);
                        }
                        Err(i2c::Error::ArbitrationLost) => {
                            self.state = State::ArbitrationLost;
                            return Err(ERR_ARBITRATION_LOST);
                        }
                        Err(_) => return Err(ERR_UNKNOWN),
                    }
                    restart = false;
                }

                Ok(0)
            }
            _ => Err(ERR_INVALID_COMMAND)
        }
    }
}

pub struct I2cSclPin<P, I>(PhantomData<(P, I)>);

impl<P: TypePin, I: i2c::StaticInstance> ResourceMode for I2cSclPin<P, I> {
    const PROTOCOL: u16 = i2c_proto::scl::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("SCL init");
        P::set_function(Function::F3);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::disable();
    }
}

pub struct I2cSdaPin<P, I>(PhantomData<(P, I)>);

impl<P: TypePin, I: i2c::StaticInstance> ResourceMode for I2cSdaPin<P, I> {
    const PROTOCOL: u16 = i2c_proto::sda::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("SDA init");
        P::set_function(Function::F3);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::disable();
    }
}
