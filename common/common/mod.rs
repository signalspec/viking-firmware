use panic_probe as _;
use defmt_rtt as _;
use zeptos::Runtime;
use zeptos::usb::Usb;
use zerocopy::{FromBytes, IntoBytes};

use core::{cell::Cell, convert::Infallible};
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
        last_config_err: Cell::new(0),
    }).await
}

#[derive(Copy, Clone)]
pub struct Resource {
    pub rt: Runtime,
    pub id: u8,
}

impl Resource {
    pub fn rt(&self) -> Runtime {
        self.rt
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn cmd(&self, command: u8) -> u8 {
        (command << 6) | self.id
    }

    pub fn evt(&self, command: u8) -> u8 {
        (command << 6) | self.id
    }

    pub fn send_event(&self, event: u8) {
        usb::EVENT_STATE.get(self.rt).borrow_mut().put(self.evt(event));
        usb::wake_event_task(self.rt);
    }

    pub fn send_event_var_len(&self, event: u8, byte: u8) {
        usb::EVENT_STATE.get(self.rt).borrow_mut().put_var_len(self.evt(event), byte);
        usb::wake_event_task(self.rt);
    }
}

pub type ErrorByte = u8;

pub fn req_from_bytes<T: IntoBytes + FromBytes + Default>(bytes: &[u8]) -> T {
    let mut v = T::default();
    for (a, b) in v.as_mut_bytes().iter_mut().zip(bytes.iter()) {
        *a = *b;
    }
    v
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
