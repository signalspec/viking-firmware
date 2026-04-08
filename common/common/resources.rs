use super::{Reader, Writer, Resource, ErrorByte};

pub trait ResourceMode: Sized {
    const PROTOCOL: u16;
    const DESCRIPTOR: &'static [u8];

    fn init(_resource: Resource, config: &[u8]) -> Result<Self, ErrorByte>;

    fn deinit(self, _resource: Resource);

    #[allow(async_fn_in_trait)]
    async fn command(&mut self, _resource: Resource, _cmd: u8, _buf: &mut Reader<'_>, _res: &mut Writer<'_>) -> Result<u8, ErrorByte> {
        Err(viking_protocol::errors::ERR_INVALID_COMMAND)
    }
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
                    pub fn deinit(self, resource: crate::common::Resource) {
                        #![allow(unused_variables)]
                        #[allow(unused_imports)]
                        use crate::common::resources::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.deinit(resource),)*
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

            fn configure(&mut self, resource: crate::common::Resource, mode: u8, config: &[u8]) -> Result<(), u8> {
                #![allow(unreachable_code)]
                match resource.id() {
                    $(
                        res_id if res_id == const { ${index()} + 1 } => {
                            if let Some(r) = self.$resource_name.take() { r.deinit(resource) }
                            self.$resource_name = Some(match mode {
                                $(mode_id if mode_id == const { ${index()} + 1 } => {
                                    resources::$resource_name::$mode_name(<$mode_ty as crate::common::resources::ResourceMode>::init(resource, config)?)
                                })*
                                _ => return Err(viking_protocol::errors::ERR_INVALID_MODE)
                            });
                            Ok(())
                        }
                    )*
                    _ => Err(viking_protocol::errors::ERR_INVALID_RESOURCE),
                }
            }

            fn reset_all(&mut self, rt: zeptos::Runtime) {
                $(
                    if let Some(r) = self.$resource_name.take() { r.deinit(crate::common::Resource { id: const { ${index()} + 1 }, rt }) }
                )*
            }

            async fn command(&mut self, resource: crate::common::Resource, command: u8, req: &mut crate::common::Reader<'_>, res: &mut crate::common::Writer<'_>) -> Result<u8, u8> {
                use crate::common::resources::ResourceMode;
                match resource.id() {
                    $(
                        res_id if res_id == const { ${index()} + 1 } => match &mut self.$resource_name {
                            $(Some(resources::$resource_name::$mode_name(s)) => s.command(resource, command, req, res).await,)*
                            _ => Err(viking_protocol::errors::ERR_INVALID_MODE)
                        }
                    )*
                    _ => Err(viking_protocol::errors::ERR_INVALID_RESOURCE),
                }
            }
        }
    }
}
