use core::{cell::Cell, marker::PhantomData, mem::take};

use atsamd_hal::{ehal::spi::Mode, gpio::{PinId, B}, pac::{interrupt, sercom0::{self, RegisterBlock, I2CM}, Interrupt, SERCOM0, SERCOM1}, sercom::Sercom};
use lilos::exec::Notify;
use defmt::{debug, info, Format};

use viking_protocol::{protocol::spi, U32};
use viking_protocol::AsBytes;

use crate::{viking::{const_bytes, take_first, take_len, ResourceMode, Writer}, viking_sam0::{pin::IoPin, sercom::DynSercom, AlternateFunc}};

pub struct SercomSPI<S, const DOPO: u8, const DIPO: u8> {
    _p: PhantomData<S>,
}

impl<S: Sercom, const DOPO: u8, const DIPO: u8> ResourceMode for SercomSPI<S, DOPO, DIPO> {
    fn describe() -> &'static [u8] {
        use spi::controller::ModeFlags;
        const_bytes!(
            spi::controller::DescribeMode {
                protocol: viking_protocol::ConstU16::new(),
                flags: ModeFlags::BYTE_AT_A_TIME,
                //base_clock: U32::new(0x48_000_000 / 2),
                //min_div: U32::new(1),
                //max_div: U32::new(256),
                //max_div_pow: 0,
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("spi init");
        init(DynSercom(S::NUM), DOPO, DIPO);
        Ok(SercomSPI { _p: PhantomData })
    }

    fn deinit(self) {
        info!("spi deinit");
        deinit(DynSercom(S::NUM));
    }

    async fn command(&self, command: u8, buf: &mut &[u8], response: &mut Writer<'_>) -> Result<(), ()> {
        use spi::controller::cmd;
        let sercom = DynSercom(S::NUM);
        
        match command {
            cmd::TRANSFER => {
                transfer(sercom, buf, response).await?;
                Ok(())
            }
            _ => Err(())
        }
    }
}

pub struct SercomSCKPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: PinId, S: Sercom, M: AlternateFunc> ResourceMode for SercomSCKPin<P, S, M> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            spi::sck_pin::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("sercom SCK init {:?} {:?}", P::DYN.group as u8, P::DYN.num);
        IoPin::<P>::alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self) {
        IoPin::<P>::reset();
    }
}

pub struct SercomSOPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: PinId, S, M: AlternateFunc> ResourceMode for SercomSOPin<P, S, M> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            spi::so_pin::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("sercom SO init {:?} {:?}", P::DYN.group as u8, P::DYN.num);
        IoPin::<P>::alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self) {
        IoPin::<P>::reset();
    }
}

pub struct SercomSIPin<P, S, M>(PhantomData<(P, S, M)>);

impl<P: PinId, S, M: AlternateFunc> ResourceMode for SercomSIPin<P, S, M> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            spi::si_pin::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("sercom SI init {:?} {:?}", P::DYN.group as u8, P::DYN.num);
        IoPin::<P>::alternate(M::DYN);
        Ok(Self(PhantomData))
    }

    fn deinit(self) {
        IoPin::<P>::reset();
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

#[inline]
fn sync_sysop(regs: &I2CM) {
    while regs.syncbusy.read().sysop().bit_is_set() {}
}

async fn transfer(sercom: DynSercom, request: &mut &[u8], response: &mut Writer<'_>) -> Result<(), ()> {
    let regs = sercom.regs().spi();

    let len = take_first(request).ok_or(())? as u8;

    for i in 0..len {
        let so_byte = take_first(request).ok_or(())?;
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


