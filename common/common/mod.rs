use panic_probe as _;
use defmt_rtt as _;
use zeptos::usb::Usb;

use core::convert::Infallible;
use core::cell::RefCell;

mod buf;
pub mod usb_descriptors;
pub mod resources;
pub mod usb;

use crate::{Resources, Platform};
pub use buf::{Writer, Reader};
pub use resources::ResourceMode;

pub async fn run(mut usb: Usb, platform: Platform) -> Infallible {
    let rt = usb.rt();
    usb.run_device(&mut usb::Handler {
        rt,
        platform,
        resources: RefCell::new(Resources::new()),
    }).await
}

#[macro_export]
macro_rules! const_bytes {
    ($($n:ident)::+ { $($inner:tt)* }) => {
        const {
            unsafe {
                &::core::mem::transmute::<_, [u8; core::mem::size_of::<$($n)::*>()]>($($n)::* {
                    $($inner)*
                })
            }
        }
    }
}
