use defmt::info;
use futures_util::future::{Fuse, FusedFuture, FutureExt};
use zeptos::usb::{UsbBuffer, In, Out, Endpoints, Setup, Responded};
use zeptos::usb::descriptors::{DescriptorBuilder, LANGUAGE_LIST_US_ENGLISH};
use zeptos::Runtime;
use core::cell::RefCell;
use core::future::poll_fn;
use core::convert::Infallible;
use core::task::Poll;
use core::mem;
use core::pin::pin;
use core::ptr::addr_of_mut;

use crate::common::{Reader, Writer};

// Board-specific configuration defined at the root of the crate
use crate::{CMD_BUF_SIZE, RES_BUF_SIZE, EVT_BUF_SIZE, PRODUCT_STRING, VIKING_DESCRIPTOR, Resources, Platform};
use crate::common::usb_descriptors::{EP_OUT, EP_IN, EP_EVT};

static mut BULK_OUT_BUF: UsbBuffer<{CMD_BUF_SIZE}> = UsbBuffer::new();
static mut BULK_IN_BUF: UsbBuffer<{RES_BUF_SIZE}> = UsbBuffer::new();
static mut EVT_IN_BUF1: UsbBuffer<{EVT_BUF_SIZE}> = UsbBuffer::new();
static mut EVT_IN_BUF2: UsbBuffer<{EVT_BUF_SIZE}> = UsbBuffer::new();

pub struct Handler {
    pub rt: Runtime,
    pub platform: Platform,
    pub resources: RefCell<Resources>,
}

impl zeptos::usb::Handler for Handler{
    fn get_descriptor<'a>(&self, kind: u8, index: u8, _lang: u16, builder: &'a mut DescriptorBuilder) -> Option<&'a [u8]> {
        use usb::descriptor_type::{CONFIGURATION, DEVICE, BOS, STRING};
        use crate::common::usb_descriptors::{DEVICE_DESCRIPTOR, CONFIG_DESCRIPTOR, BOS_DESCRIPTOR, MANUFACTURER_STRING, STRING_MFG, STRING_PRODUCT, STRING_SERIAL};

        match (kind, index) {
            (DEVICE, _) => Some(DEVICE_DESCRIPTOR),
            (CONFIGURATION, 0) => Some(CONFIG_DESCRIPTOR),
            (BOS, 0) => Some(BOS_DESCRIPTOR),
            (STRING, 0) => Some(LANGUAGE_LIST_US_ENGLISH),
            (STRING, STRING_MFG) => Some(builder.string_ascii(MANUFACTURER_STRING)),
            (STRING, STRING_PRODUCT) => Some(builder.string_ascii(PRODUCT_STRING)),
            (STRING, STRING_SERIAL) => Some(builder.string_hex(&zeptos::serial_number())),
            _ => None,
        }
    }

    async fn set_configuration(&self, cfg: u8, usb: &mut Endpoints) -> Result<(), ()> {
        if cfg == 1 {
            self.set_interface(0, 0, usb).await
        } else {
            Err(())
        }
    }

    async fn set_interface(&self, intf: u8, alt: u8, usb: &mut Endpoints) -> Result<(), ()> {
        if intf == 0 {
            bulk_task(self.rt).cancel();
            evt_task(self.rt).cancel();

            self.resources.borrow_mut().reset_all();

            if alt == 1 {
                info!("Enabling interface");
                let ep_out = usb.bulk_out::<EP_OUT>();
                let ep_in = usb.bulk_in::<EP_IN>();
                let ep_evt = usb.bulk_in::<EP_EVT>();

                // usb.run never exits, so `self` lasts for static.
                let resources = unsafe { core::mem::transmute::<&_, &'static _>(&self.resources) };

                bulk_task(self.rt).spawn(self.rt, resources, ep_out, ep_in);
                evt_task(self.rt).spawn(self.rt, resources, ep_evt);
            } else {
                info!("Disabled interface");
            }

            Ok(())
        } else {
            Err(())
        }
    }

    async fn handle_control<'a>(&self, req: Setup<'a>) -> Responded {
        use zeptos::usb::ControlType::*;
        use zeptos::usb::Recipient::*;
        use zeptos::usb::ControlData::{In, Out};

        use crate::common::usb_descriptors::{INTF_VIKING, MSOS_VENDOR_CODE, MSOS_DESCRIPTOR};
        const I_VIKING: u16 = INTF_VIKING as u16;

        use viking_protocol::request::{DESCRIBE_RESOURCES, CONFIGURE_MODE};

        match req {
            Setup { ty: Vendor, recipient: Device, request: MSOS_VENDOR_CODE, index: 0x07, data: In(data), .. } => {
                data.respond(&MSOS_DESCRIPTOR).await
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, request: DESCRIBE_RESOURCES, data: In(data), .. } => {
                data.respond(VIKING_DESCRIPTOR).await
            }

            Setup { ty: Vendor, recipient: Interface, index: I_VIKING, value, request: CONFIGURE_MODE, data: Out(data), .. } => {
                let resource = (value >> 8) as u8;
                let mode = (value & 0xff) as u8;
                info!("configure {} {}", resource, mode);

                let ok = if let Ok(mut resources) = self.resources.try_borrow_mut() {
                    resources.configure(resource, mode, &[]).is_ok()
                } else {
                    info!("resource busy");
                    false
                };

                if ok {
                    data.accept().await
                } else {
                    data.reject()
                }
            }

            unknown => unknown.reject(),
        }
    }
}

#[zeptos::task]
async fn bulk_task(rt: Runtime, resources: &'static RefCell<Resources>, mut ep_out: zeptos::usb::Endpoint<Out, EP_OUT>, mut ep_in: zeptos::usb::Endpoint<In, EP_IN>) {
    loop {
        let buf_out = unsafe { &mut *addr_of_mut!(BULK_OUT_BUF) };
        let buf_in = unsafe { &mut *addr_of_mut!(BULK_IN_BUF) };

        loop {
            let len = ep_out.receive(buf_out).await;
            info!("bulk read {}: {:?}", len, &buf_out[..len]);

            if len < 2 {
                continue;
            }
            let sync = buf_out[0];

            let req = Reader::new(&buf_out[2..len]);
            let mut res = Writer::new(&mut buf_in[..], 2);

            let status = match run_cmds(rt, &mut resources.borrow_mut(), req, &mut res).await {
                Ok(_) => 0,
                Err(_) => 1,
            };

            let response_len = res.offset();
            buf_in[0] = sync;
            buf_in[1] = status;
            ep_in.send(buf_in, response_len, true).await; //todo zlp
            info!("bulk write complete");
        }
    }
}

async fn run_cmds(rt: Runtime, resources: &mut Resources, mut req: Reader<'_>, res: &mut Writer<'_>) -> Result<(), ()> {
    while let Some(byte) = req.take_first() {
        use viking_protocol::protocol::cmd;
        match byte {
            cmd::DELAY => {
                let us: u32 = req.take_u16().ok_or(())? as u32;
                rt.delay_us(us).await;
            }
            byte => {
                let resource = byte & ((1 << 6) - 1);
                let command = byte >> 6;
                resources.command(rt, resource, command, &mut req, res).await?;
            }
        }
    }

    Ok(())
}

#[zeptos::task]
async fn evt_task(_rt: Runtime, resources: &'static RefCell<Resources>, ep_evt: zeptos::usb::Endpoint<In, EP_EVT>) {
    let ep_evt = RefCell::new(ep_evt);
    let mut buf_fill = &raw mut EVT_IN_BUF1;
    let mut buf_send = &raw mut EVT_IN_BUF2;

    loop {
        let mut transfer = pin!(Fuse::terminated());
        let mut buf = Writer::new(unsafe { &mut (&mut *buf_fill)[..] }, 0);

        poll_fn(|cx| -> Poll<Infallible> {
            //EVENT_CHANGE.subscribe(cx.waker());
            resources.borrow().poll_all(cx.waker(), &mut buf);

            let _ = transfer.as_mut().poll(cx);
            info!("Events: {:?} {} {:?}", buf_fill, buf.offset(), transfer.is_terminated());

            if transfer.is_terminated() && buf.offset() > 0 {
                let len = buf.offset();
                buf = Writer::new(unsafe { &mut (&mut *buf_send)[..] }, 0);
                info!("Sending events: {}", len);
                let mut ep_evt = ep_evt.borrow_mut();
                transfer.set(async move { ep_evt.send(unsafe { &mut *buf_fill }, len, true).await }.fuse());
                let _ = transfer.as_mut().poll(cx);
                mem::swap(&mut buf_fill, &mut buf_send);
            }

            Poll::Pending
        }).await;
    }
}
