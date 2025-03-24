#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(macro_metavar_expr)]
#![feature(inline_const_pat)]

use panic_probe as _;
use defmt_rtt as _;

use core::task::Waker;

mod buf;
pub use buf::{Writer, Reader};

#[doc(hidden)]
pub mod usb_descriptors;

#[cfg(feature = "rp2040")]
pub mod rp;

#[cfg(any(feature = "samd11", feature = "samd21"))]
pub mod sam0;

pub trait Platform {
    
}

pub trait ResourceMode: Sized {
    const PROTOCOL: u16;
    const DESCRIPTOR: &'static [u8];

    fn init(config: &[u8]) -> Result<Self, ()>;

    fn deinit(self);

    #[allow(async_fn_in_trait)]
    async fn command(&self, _cmd: u8, _buf: &mut Reader<'_>, _res: &mut Writer<'_>) -> Result<(), ()> {
        Err(())
    }

    fn poll_event(&self, _waker: &Waker, _resource: u8, _buf: &mut Writer<'_>) {}
}

#[doc(hidden)]
pub mod deps {
    // Dependency re-exports for use in the macro
    pub use usb;
    pub use futures_util;
    pub use defmt;
    pub use viking_protocol;
}

#[macro_export]
macro_rules! viking{
    (
        $mod_name:ident<$platform_ty:ty> {
            $(const $name:ident: $ty:ty = $val:expr;)*
            $(
                resource $resource_name:ident {
                    $($mode_name:ident : $mode_ty:ty,)*
                }
            )*
        }
    ) => {
        pub mod $mod_name {
            use $crate::{self as viking};
            use super::*;

            use core::cell::RefCell;
            use core::convert::Infallible;
            use core::future::{Future, poll_fn};
            use core::mem;
            use core::pin::pin;
            use core::ptr::addr_of_mut;
            use core::task::{Poll, Waker};
            use $crate::deps::futures_util::future::{Fuse, FusedFuture};
            use $crate::deps::futures_util::FutureExt;
            use $crate::deps::defmt::info;
            use $crate::deps::viking_protocol;
            use $crate::deps::usb;
            use zeptos::usb::descriptors::{descriptors, BinaryObjectStore, Config, DescriptorBuilder, Device, Endpoint, Interface, MicrosoftOs, MicrosoftOsCompatibleID, PlatformCapabilityMicrosoftOs, LANGUAGE_LIST_US_ENGLISH };
            use zeptos::usb::{Usb, Endpoints, In, Out, Responded, Setup, UsbBuffer};
            use zeptos::Runtime;
            use zeptos::cortex_m::SysTick;
            use $crate::{Writer, Reader};
            use $crate::usb_descriptors::{EP_IN, EP_OUT, EP_EVT};
            use $crate::deps::viking_protocol::{U32, U16};

            $(const $name: $ty = $val;)*

            pub async fn run(mut usb: Usb, systick: SysTick, platform: $platform_ty) -> Infallible {
                let rt = usb.rt();;
                usb.run_device(&mut Handler {
                    rt,
                    systick: RefCell::new(systick),
                    platform,
                    resources: RefCell::new(Resources::new()),
                }).await
            }

            mod resources {
                use super::*;
                
                $(
                    #[allow(non_camel_case_types)]
                    pub enum $resource_name {
                        $($mode_name($mode_ty)),*
                    }
    
                    impl $resource_name {
                        pub fn deinit(self) {
                            use viking::ResourceMode;
                            match self {
                                $(Self::$mode_name(s) => s.deinit(),)*
                            }
                        }
                    }
            
                )*
            }

            const VIKING_DESCRIPTOR: &'static [u8] = const {
                use viking_protocol::descriptor::*;
                const PARTS: &[(u8, Option<u16>, &[u8])] = &[
                    (DESCRIPTOR_TYPE_VIKING, None, viking_firmware_common::const_bytes!(
                        VikingDescriptor {
                            total_len: U16::new(0), // filled below
                            version: 0x01,
                            rsvd: 0x00,
                            max_cmd: U32::new(CMD_BUF_SIZE as u32),
                            max_res: U32::new(RES_BUF_SIZE as u32),
                            max_evt: U32::new(EVT_BUF_SIZE as u32),
                        }
                    )),
                    $(
                        (DESCRIPTOR_TYPE_RESOURCE, None, &[]),
                        (DESCRIPTOR_TYPE_IDENTIFIER, None, stringify!($resource_name).as_bytes()),
                        $(
                            (DESCRIPTOR_TYPE_MODE,
                                Some(<$mode_ty as viking::ResourceMode>::PROTOCOL),
                                <$mode_ty as viking::ResourceMode>::DESCRIPTOR
                            ),
                            (DESCRIPTOR_TYPE_IDENTIFIER, None, stringify!($mode_name).as_bytes()),
                        )*
                    )*
                ];

                const LEN: usize = {
                    let mut len = 0;
                    let mut i = 0;
                    while i < PARTS.len() {
                        len += 2 + 2 * PARTS[i].1.is_some() as usize + PARTS[i].2.len();
                        i += 1;
                    }
                    len
                };

                &const {
                    let mut bytes = [0; LEN];
                    let mut pos = 0;
                    let mut part = 0;

                    while part < PARTS.len() {
                        let p = &PARTS[part];

                        let len = 2 + 2 * p.1.is_some() as usize + p.2.len();
                        assert!(len < u8::MAX as usize);

                        bytes[pos + 0] = len as u8;
                        bytes[pos + 1] = p.0;
                        pos += 2;

                        if let Some(n) = p.1 {
                            bytes[pos + 0] = n.to_le_bytes()[0];
                            bytes[pos + 1] = n.to_le_bytes()[1];
                            pos += 2;
                        }

                        let mut i = 0;
                        while i < p.2.len() {
                            bytes[pos + i] = p.2[i];
                            i += 1;
                        }
                        pos += p.2.len();

                        part += 1;
                    }

                    assert!(pos == LEN);

                    // fill total length
                    bytes[2] = (pos as u16).to_le_bytes()[0];
                    bytes[3] = (pos as u16).to_le_bytes()[1];

                    bytes
                }
            };

            pub struct Resources {
                $(
                    $resource_name: Option<resources::$resource_name>,
                )*
            }

            impl Resources {
                fn new() -> Self {
                    Self {
                        $(
                            $resource_name: None,
                        )*
                    }
                }

                fn configure(&mut self, resource: u8, mode: u8, config: &[u8]) -> Result<(), ()> {
                    #![allow(unreachable_code)]
                    match resource {
                        $(
                            const { ${index()} + 1 } => {
                                if let Some(r) = self.$resource_name.take() { r.deinit() }
                                self.$resource_name = Some(match mode {
                                    $(const { ${index()} + 1 } => resources::$resource_name::$mode_name(<$mode_ty as viking::ResourceMode>::init(config)?),)*
                                    _ => return Err(())
                                });
                                Ok(())
                            }
                        )*
                        _ => Err(()),
                    }
                }

                fn reset_all(&mut self) {
                    $(
                        if let Some(r) = self.$resource_name.take() { r.deinit() }
                    )*
                }

                async fn run(&self, mut systick: &mut SysTick, mut req: Reader<'_>, res: &mut Writer<'_>) -> Result<(), ()> {
                    while let Some(byte) = req.take_first() {
                        use viking_protocol::protocol::cmd::DELAY;
                        match byte {
                            DELAY => {
                                let mut us: u32 = req.take_varint().ok_or(())?;
                                if us > 0xFFFF {
                                    info!("Delay too long");
                                    return Err(());
                                }
                                systick.delay_us(us).await;
                            }
                            byte => {
                                let resource = byte & ((1 << 6) - 1);
                                let command = byte >> 6;
                                self.command(resource, command, &mut req, res).await?;
                            }
                        }
                    }

                    Ok(())
                }

                async fn command(&self, resource: u8, command: u8, req: &mut Reader<'_>, res: &mut Writer<'_>) -> Result<(), ()> {
                    use viking::ResourceMode;
                    match resource {
                        $(
                            const { ${index()} + 1 } => match &self.$resource_name {
                                $(Some(resources::$resource_name::$mode_name(s)) => s.command(command, req, res).await,)*
                                _ => Err(())
                            }
                        )*
                        _ => Err(())
                    }
                }

                fn poll_all(&self, waker: &core::task::Waker, buf: &mut viking::Writer) {
                    use viking::ResourceMode;
                    $(
                        match &self.$resource_name {
                            $(Some(resources::$resource_name::$mode_name(s)) => s.poll_event(waker, ${index()} + 1, buf),)*
                            _ => {}
                        }
                    )*
                }
            }
        

            static mut BULK_OUT_BUF: UsbBuffer<{CMD_BUF_SIZE}> = UsbBuffer::new();
            static mut BULK_IN_BUF: UsbBuffer<{RES_BUF_SIZE}> = UsbBuffer::new();
            static mut EVT_IN_BUF1: UsbBuffer<{EVT_BUF_SIZE}> = UsbBuffer::new();
            static mut EVT_IN_BUF2: UsbBuffer<{EVT_BUF_SIZE}> = UsbBuffer::new();

            struct Handler {
                rt: Runtime,
                systick: RefCell<SysTick>,
                platform: Platform,
                resources: RefCell<Resources>,
            }
            
            impl zeptos::usb::Handler for Handler {
                fn get_descriptor<'a>(&self, kind: u8, index: u8, _lang: u16, builder: &'a mut DescriptorBuilder) -> Option<&'a [u8]> {
                    use usb::descriptor_type::{CONFIGURATION, DEVICE, BOS, STRING};
                    use $crate::usb_descriptors::{DEVICE_DESCRIPTOR, CONFIG_DESCRIPTOR, BOS_DESCRIPTOR, MANUFACTURER_STRING, STRING_MFG, STRING_PRODUCT, STRING_SERIAL};
            
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
                            let systick = unsafe { core::mem::transmute::<&_, &'static _>(&self.systick) };
                            let resources = unsafe { core::mem::transmute::<&_, &'static _>(&self.resources) };
                
                            bulk_task(self.rt).spawn(systick, resources, ep_out, ep_in);
                            evt_task(self.rt).spawn(resources, ep_evt);
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
            
                    use $crate::usb_descriptors::{INTF_VIKING, MSOS_VENDOR_CODE, MSOS_DESCRIPTOR};
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
            async fn bulk_task(systick: &'static RefCell<SysTick>, resources: &'static RefCell<Resources>, mut ep_out: zeptos::usb::Endpoint<Out, EP_OUT>, mut ep_in: zeptos::usb::Endpoint<In, EP_IN>) {
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
                        
                        let status = match resources.borrow().run(&mut *systick.borrow_mut(), req, &mut res).await {
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
            
            #[zeptos::task]
            async fn evt_task(resources: &'static RefCell<Resources>, ep_evt: zeptos::usb::Endpoint<In, EP_EVT>) {
                let ep_evt = RefCell::new(ep_evt);
                let mut buf_fill = &raw mut EVT_IN_BUF1;
                let mut buf_send = &raw mut EVT_IN_BUF2;
                
                loop {
                    let mut transfer = pin!(Fuse::terminated());
                    let mut buf = Writer::new(unsafe { &mut (*buf_fill)[..] }, 0);
            
                    poll_fn(|cx| -> Poll<Infallible> {
                        //EVENT_CHANGE.subscribe(cx.waker());
                        resources.borrow().poll_all(cx.waker(), &mut buf);
            
                        let _ = transfer.as_mut().poll(cx);
                        info!("Events: {:?} {} {:?}", buf_fill, buf.offset(), transfer.is_terminated());
            
                        if transfer.is_terminated() && buf.offset() > 0 {
                            let len = buf.offset();
                            buf = Writer::new(unsafe { &mut (*buf_send)[..] }, 0);
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
        }
    }
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
