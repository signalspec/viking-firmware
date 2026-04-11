use core::{marker::PhantomData};

use zeptos::rp::{gpio::{TypePin, Function}, spi};
use defmt::{debug, info};

use viking_protocol::{errors::{ERR_INVALID_COMMAND, ERR_MISSING_ARG, ERR_RESPONSE_FULL, ERR_UNSUPPORTED_CLOCK}, protocol::spi as spi_proto};

use crate::{common::ErrorByte, const_bytes};
use crate::common::{Reader, Resource, ResourceMode, Writer, req_from_bytes};


pub struct Spi<I: spi::StaticInstance> {
    controller: spi::Controller<I>,
}

impl<I: spi::StaticInstance> ResourceMode for Spi<I> {
    const PROTOCOL: u16 = spi_proto::controller::PROTOCOL;
    const DESCRIPTOR: &[u8] = {
        use spi_proto::controller::ModeFlags;
        use viking_protocol::U32;
        const_bytes!(
            spi_proto::controller::DescribeMode {
                flags: ModeFlags::PINS
                    .union(ModeFlags::MODE0)
                    .union(ModeFlags::MODE1)
                    .union(ModeFlags::MODE2)
                    .union(ModeFlags::MODE3)
                    .union(ModeFlags::MSB_FIRST),
                base_clock: U32::new(spi::Config::BASE_CLOCK_HZ),
                max_div: U32::new(spi::Config::MAX_DIV),
            }
        )
    };

    fn init(_resource: Resource, req: &[u8]) -> Result<Self, u8> {
        info!("spi init");
        let req = req_from_bytes::<spi_proto::controller::Config>(req);
        let mut config = spi::Config::default();
        let div = req.clock_div.get();
        config.set_divisor(if div != 0 { div } else { spi::Config::BASE_CLOCK_HZ / 1_000_000 }).map_err(|()| ERR_UNSUPPORTED_CLOCK)?;
        config.mode = req.flags.mode();
        let instance = unsafe { I::steal() };
        let controller = spi::Controller::new(instance, config);
        Ok(Spi { controller })
    }

    fn deinit(self, _resource: Resource) {
        info!("spi deinit");
    }

    async fn command(&mut self, _resource: Resource, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<u8, ErrorByte> {
        use spi_proto::controller::cmd;

        match command {
            cmd::WRITE => {
                let buf = req.take_len().ok_or(ERR_MISSING_ARG)?;
                debug!("SPI write {} bytes", buf.len());
                self.controller.transfer(buf.iter().copied(), ()).await;
                Ok(0)
            }
            cmd::READ => {
                let len = req.take_first().ok_or(ERR_MISSING_ARG)? as usize;
                let out = res.reserve_buf(len).map_err(|()| ERR_RESPONSE_FULL)?;
                debug!("SPI read {} bytes", len);
                self.controller.transfer(core::iter::repeat_n(0, len), out).await;
                Ok(0)
            }
            cmd::TRANSFER => {
                let buf = req.take_len().ok_or(ERR_MISSING_ARG)?;
                let out = res.reserve_buf(buf.len()).map_err(|()| ERR_RESPONSE_FULL)?;
                debug!("SPI transfer {} bytes", buf.len());
                self.controller.transfer(buf.iter().copied(), out).await;
                Ok(0)
            }
            _ => Err(ERR_INVALID_COMMAND)
        }
    }
}

pub struct SpiSckPin<P, I>(PhantomData<(P, I)>);

impl<P: TypePin, I: spi::StaticInstance> ResourceMode for SpiSckPin<P, I> {
    const PROTOCOL: u16 = spi_proto::sck_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("SCK init");
        P::set_function(Function::F1);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::disable();
    }
}

pub struct SpiSdoPin<P, I>(PhantomData<(P, I)>);

impl<P: TypePin, I: spi::StaticInstance> ResourceMode for SpiSdoPin<P, I> {
    const PROTOCOL: u16 = spi_proto::sdo_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("SDO init");
        P::set_function(Function::F1);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::disable();
    }
}


pub struct SpiSdiPin<P, I>(PhantomData<(P, I)>);

impl<P: TypePin, I: spi::StaticInstance> ResourceMode for SpiSdiPin<P, I> {
    const PROTOCOL: u16 = spi_proto::sdi_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ErrorByte> {
        info!("SDI init");
        P::set_function(Function::F1);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::disable();
    }
}
