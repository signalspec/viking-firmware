use core::marker::PhantomData;

use super::pin::{PinId, IoPin};
use defmt::info;
use viking_protocol::protocol::gpio::pin::DescribeMode;

use crate::viking::{ResourceMode, Writer};
use viking_protocol::AsBytes;

pub struct Gpio<P>(PhantomData<P>);

use viking_protocol::protocol::gpio;

macro_rules! const_bytes {
    ($($n:ident)::+ { $($inner:tt)* }) => {
        {
            static S: $($n)::* = $($n)::* {
                $($inner)*
            };
            S.as_bytes()
        }
    }
}


impl<P: PinId> ResourceMode for Gpio<P> {
    fn describe() -> &'static [u8] {
        const_bytes!(
            gpio::pin::DescribeMode {
                protocol: viking_protocol::ConstU16::new()
            }
        )
    }

    fn init(config: &[u8]) -> Result<Self, ()> {
        info!("gpio init {:?} {:?}", P::DYN.group as u8, P::DYN.num);
        IoPin::<P>::pincfg().write(|w| w.inen().set_bit());
        IoPin::<P>::enable_sampling();
        Ok(Gpio(PhantomData))
    }

    fn deinit(self) {
        info!("gpio deinit");
        IoPin::<P>::dirclr();
    }

    async fn command(&self, command: u8, buf: &mut &[u8], response: &mut Writer<'_>) -> Result<(), ()> {
        use viking_protocol::protocol::gpio::pin::cmd;
        
        match command {
            cmd::FLOAT => {
                IoPin::<P>::dirclr();
                Ok(())
            }
            cmd::READ => {
                let byte: u8 = if IoPin::<P>::read() { 0x01 } else { 0x00 };
                response.put(byte)?;
                Ok(())
            }
            cmd::LOW => {
                IoPin::<P>::outclr();
                IoPin::<P>::dirset();
                Ok(())
            }
            cmd::HIGH => {
                IoPin::<P>::outset();
                IoPin::<P>::dirset();
                Ok(())
            }
            _ => Err(())
        }
    }
}
