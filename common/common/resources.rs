use core::task::Waker;

use zeptos::Runtime;
use super::{Reader, Writer};

pub trait ResourceMode: Sized {
    const PROTOCOL: u16;
    const DESCRIPTOR: &'static [u8];

    fn init(config: &[u8]) -> Result<Self, ()>;

    fn deinit(self);

    #[allow(async_fn_in_trait)]
    async fn command(&self, _rt: Runtime, _cmd: u8, _buf: &mut Reader<'_>, _res: &mut Writer<'_>) -> Result<(), ()> {
        Err(())
    }

    fn poll_event(&self, _waker: &Waker, _resource: u8, _buf: &mut Writer<'_>) {}
}

#[macro_export]
macro_rules! viking{
    (
        $(
            resource $resource_name:ident {
                $($mode_name:ident : $mode_ty:ty,)*
            }
        )*
    ) => {
        mod resources {
            use super::*;

            $(
                #[allow(non_camel_case_types)]
                pub enum $resource_name {
                    $($mode_name($mode_ty)),*
                }

                impl $resource_name {
                    pub fn deinit(self) {
                        #[allow(unused_imports)]
                        use crate::common::resources::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.deinit(),)*
                        }
                    }
                }

            )*
        }

        const VIKING_DESCRIPTOR: &'static [u8] = const {
            use viking_protocol::{U16, U32};
            use viking_protocol::descriptor::*;
            const PARTS: &[(u8, Option<u16>, &[u8])] = &[
                (DESCRIPTOR_TYPE_VIKING, None, const_bytes!(
                    VikingDescriptor {
                        total_len: U16::new(0), // filled below
                        version: 0x01,
                        rsvd: 0x00,
                        max_cmd: U32::new($crate::CMD_BUF_SIZE as u32),
                        max_res: U32::new($crate::RES_BUF_SIZE as u32),
                        max_evt: U32::new($crate::EVT_BUF_SIZE as u32),
                    }
                )),
                $(
                    (DESCRIPTOR_TYPE_RESOURCE, None, &[]),
                    (DESCRIPTOR_TYPE_IDENTIFIER, None, stringify!($resource_name).as_bytes()),
                    $(
                        (DESCRIPTOR_TYPE_MODE,
                            Some(<$mode_ty as crate::common::resources::ResourceMode>::PROTOCOL),
                            <$mode_ty as crate::common::resources::ResourceMode>::DESCRIPTOR
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

        pub struct Resources{
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
                        res_id if res_id == const { ${index()} + 1 } => {
                            if let Some(r) = self.$resource_name.take() { r.deinit() }
                            self.$resource_name = Some(match mode {
                                $(mode_id if mode_id == const { ${index()} + 1 } => resources::$resource_name::$mode_name(<$mode_ty as crate::common::resources::ResourceMode>::init(config)?),)*
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

            async fn command(&self, rt: zeptos::Runtime, resource: u8, command: u8, req: &mut crate::common::Reader<'_>, res: &mut crate::common::Writer<'_>) -> Result<(), ()> {
                use crate::common::resources::ResourceMode;
                match resource {
                    $(
                        res_id if res_id == const { ${index()} + 1 } => match &self.$resource_name {
                            $(Some(resources::$resource_name::$mode_name(s)) => s.command(rt, command, req, res).await,)*
                            _ => Err(())
                        }
                    )*
                    _ => Err(())
                }
            }

            fn poll_all(&self, waker: &core::task::Waker, buf: &mut crate::common::Writer) {
                use crate::common::resources::ResourceMode;
                $(
                    #[allow(unused_variables)]
                    let res_id = const { ${index()} + 1 };
                    match &self.$resource_name {
                        $(Some(resources::$resource_name::$mode_name(s)) => s.poll_event(waker, res_id, buf),)*
                        _ => {}
                    }
                )*
            }
        }
    }
}
