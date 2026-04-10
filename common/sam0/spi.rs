use core::marker::PhantomData;

use zeptos::samd::gpio::{AlternateFunc, TypePin};
use zeptos::samd::sercom::{Sercom, StaticSercom, SpiController, SpiConfig};
use defmt::info;

use viking_protocol::{protocol::spi as spi_proto, U32};
use viking_protocol::errors::{ERR_INVALID_COMMAND, ERR_MISSING_ARG};

use crate::const_bytes;
use crate::common::{Reader, Resource, ResourceMode, Writer, ErrorByte, req_from_bytes};

pub struct SercomSPI<S: StaticSercom, const DOPO: u8, const DIPO: u8> {
    spi: SpiController<S>,
}

impl<S: StaticSercom, const DOPO: u8, const DIPO: u8> ResourceMode for SercomSPI<S, DOPO, DIPO> {
    const PROTOCOL: u16 = spi_proto::controller::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = {
        use spi_proto::controller::ModeFlags;
        const_bytes!(
            spi_proto::controller::DescribeMode {
                flags: ModeFlags::MODE0
                    .union(ModeFlags::MODE1)
                    .union(ModeFlags::MODE2)
                    .union(ModeFlags::MODE3)
                    .union(ModeFlags::MSB_FIRST),
                base_clock: U32::new(SpiConfig::BASE_CLOCK),
                max_div: U32::new(256),
            }
        )
    };

    fn init(_resource: Resource, req: &[u8]) -> Result<Self, ErrorByte> {
        use spi_proto::controller::ConfigFlags;
        info!("spi init");
        let req = req_from_bytes::<spi_proto::controller::Config>(req);
        let sercom = unsafe { S::steal() };
        let clkdiv = req.clock_div.get();
        let config = SpiConfig {
            mode: req.flags.mode(),
            clkdiv_minus_one: if clkdiv == 0 {
                (SpiConfig::BASE_CLOCK / 1_000_000 - 1) as u8
            } else {
                (clkdiv - 1).min(255) as u8
            },
            dopo: DOPO,
            dipo: DIPO,
        };
        let spi = SpiController::new(sercom, config);
        Ok(SercomSPI { spi })
    }

    fn deinit(self, _resource: Resource) {
        info!("spi deinit");
    }

    async fn command(&mut self, _resource: Resource, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<u8, ErrorByte> {
        use spi_proto::controller::cmd;

        match command {
            cmd::TRANSFER | cmd::READ | cmd::WRITE => {
                let len = req.take_first().ok_or(ERR_MISSING_ARG)?;

                for _ in 0..len {
                    let tx_byte = if command == cmd::READ { 0 } else {
                        req.take_first().ok_or(ERR_MISSING_ARG)?
                    };
                    let rx_byte = self.spi.transfer(tx_byte).await;
                    if command != cmd::WRITE {
                        res.put(rx_byte)?;
                    }
                }
                Ok(0)
            }
            _ => Err(ERR_INVALID_COMMAND)
        }
    }
}

pub struct SercomSCKPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S: Sercom, M: AlternateFunc> ResourceMode for SercomSCKPin<P, S, M> {
    const PROTOCOL: u16 = spi_proto::sck_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("sercom SCK init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::set_io();
    }
}

pub struct SercomSDOPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S, M: AlternateFunc> ResourceMode for SercomSDOPin<P, S, M> {
    const PROTOCOL: u16 = spi_proto::sdo_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("sercom SDO init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::set_io();
    }
}

pub struct SercomSDIPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S, M: AlternateFunc> ResourceMode for SercomSDIPin<P, S, M> {
    const PROTOCOL: u16 = spi_proto::sdi_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("sercom SDI init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::set_io();
    }
}
