use core::marker::PhantomData;

use zeptos::{samd::gpio::{AlternateFunc, TypePin}};
use defmt::info;

use viking_protocol::{protocol::spi, U32};

use crate::const_bytes;
use crate::common::{Reader, Resource, ResourceMode, Writer};
use super::sercom::{ DynSercom, Sercom };

pub struct SercomSPI<S, const DOPO: u8, const DIPO: u8> {
    _p: PhantomData<S>,
}

impl<S: Sercom, const DOPO: u8, const DIPO: u8> ResourceMode for SercomSPI<S, DOPO, DIPO> {
    const PROTOCOL: u16 = spi::controller::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = {
        use spi::controller::ModeFlags;
        const_bytes!(
            spi::controller::DescribeMode {
                flags: ModeFlags::MODE0.union(ModeFlags::MSB_FIRST),
                base_clock: U32::new(0x48_000_000 / 2),
                min_div: U32::new(1),
                max_div: U32::new(256),
                max_div_pow: 0,
            }
        )
    };

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
        info!("spi init");
        init(DynSercom(S::NUM), DOPO, DIPO);
        Ok(SercomSPI { _p: PhantomData })
    }

    fn deinit(self, _resource: Resource) {
        info!("spi deinit");
        deinit(DynSercom(S::NUM));
    }

    async fn command(&mut self, _resource: Resource, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<(), ()> {
        use spi::controller::cmd;
        let sercom = DynSercom(S::NUM);

        match command {
            cmd::TRANSFER => {
                transfer(sercom, req, res).await?;
                Ok(())
            }
            _ => Err(())
        }
    }
}

pub struct SercomSCKPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: TypePin, S: Sercom, M: AlternateFunc> ResourceMode for SercomSCKPin<P, S, M> {
    const PROTOCOL: u16 = spi::sck_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
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
    const PROTOCOL: u16 = spi::sdo_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
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
    const PROTOCOL: u16 = spi::sdi_pin::PROTOCOL;
    const DESCRIPTOR: &'static [u8] = &[];

    fn init(_resource: Resource, _config: &[u8]) -> Result<Self, ()> {
        info!("sercom SDI init {:?} {:?}", P::DYN.group, P::DYN.pin);
        P::set_alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self, _resource: Resource) {
        P::set_io();
    }
}

fn init(sercom: DynSercom, dopo: u8, dipo: u8) {
    let regs = sercom.regs().spi();
    regs.ctrla.write(|w| w.mode().spi_master());
    regs.baud.write(|w| w.baud().variant(23) ); // 1MHz
    regs.ctrlb.write(|w| {
        w.rxen().set_bit()
    });
    regs.ctrla.write(|w| {
        w.mode().spi_master();
        w.dopo().variant(dopo);
        w.dipo().variant(dipo);
        w.enable().set_bit()
    });
    while regs.syncbusy.read().enable().bit_is_set() {}
}

fn deinit(sercom: DynSercom) {
    sercom.regs().spi().ctrla.write(|w| w.swrst().set_bit());
}

async fn transfer(sercom: DynSercom, request: &mut Reader<'_>, response: &mut Writer<'_>) -> Result<(), ()> {
    let regs = sercom.regs().spi();

    let len = request.take_first().ok_or(())? as u8;

    for _ in 0..len {
        let so_byte = request.take_first().ok_or(())?;
        regs.data.write(|w| w.data().variant(so_byte as u16));

        regs.intenset.write(|w| { w.txc().set_bit() });
        sercom.notify().until(|| {
            regs.intflag.read().txc().bit_is_set()
        }).await;

        let si_byte = regs.data.read().data().bits() as u8;
        response.put(si_byte)?;
    }

    Ok(())
}
