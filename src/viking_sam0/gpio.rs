use core::marker::PhantomData;

use super::pin::{PinId, IoPin};
use defmt::info;
use viking_protocol::protocol::gpio::pin::DescribeMode;

use crate::viking::ResourceMode;
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
        Ok(Gpio(PhantomData))
    }

    fn deinit(self) {
        info!("gpio deinit");
        IoPin::<P>::dirclr();
    }

    async fn command(&self, command: u8, buf: &mut &[u8]) -> Result<(), ()> {
        use viking_protocol::protocol::gpio::pin::cmd;
        
        match command {
            cmd::FLOAT => {
                IoPin::<P>::dirclr();
                Ok(())
            }
            cmd::READ => {
                Err(())
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
