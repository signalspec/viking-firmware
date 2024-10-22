use core::task::Waker;

use defmt::info;

use crate::delay::AsyncDelayUs;

pub struct Writer<'a> {
    offset: usize,
    buf: &'a mut [u8],
}

impl<'a> Writer<'a> {
    pub fn new(buf: &'a mut [u8], offset: usize) -> Writer {
        Writer { buf, offset }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn put(&mut self, b: u8) -> Result<(), ()> {
        let next = self.buf.get_mut(self.offset).ok_or(())?;
        *next = b;
        self.offset += 1;
        Ok(())
    }
}

pub trait ResourceMode: Sized {
    const PROTOCOL: u16;
    const DESCRIPTOR: &'static [u8];

    fn init(config: &[u8]) -> Result<Self, ()>;

    fn deinit(self);

    async fn command(&self, cmd: u8, buf: &mut &[u8], res: &mut Writer<'_>) -> Result<(), ()> {
        Err(())
    }

    fn poll_event(&self, waker: &Waker, resource: u8, buf: &mut Writer<'_>) {}
}


pub fn take_first<'a>(buf: &mut &'a [u8]) -> Option<u8> {
    let (first, rem) = buf.split_first()?;
    *buf = rem;
    Some(*first)
}

pub fn take_len<'a>(buf: &mut &'a [u8]) -> Option<&'a [u8]> {
    let len = take_first(buf)? as usize;
    let s = buf.get(..len)?;
    *buf = &buf[len..];
    Some(s)
}

pub trait Resources: Sized {
    const DESCRIPTOR: &'static [u8];

    fn new() -> Self;
    fn configure(&mut self, resource: u8, mode: u8, config: &[u8]) -> Result<(), ()>;
    fn reset_all(&mut self);
    async fn command(&self, resource: u8, command: u8, buf: &mut &[u8], response: &mut Writer) -> Result<(), ()> ;
    fn poll_all(&self, waker: &Waker, buf: &mut Writer<'_>);

    async fn run<D: AsyncDelayUs>(&self, request: &[u8], response: &mut Writer<'_>, delay: &mut D) -> Result<(), ()> {
        let mut request = request;

        while let Some(byte) = take_first(&mut request) {
            use viking_protocol::protocol::cmd::DELAY;
            match byte {
                DELAY => {
                    let mut us: u32 = 0;
                    loop {
                        let mut b = take_first(&mut request).ok_or(())?;
                        us = (us << 7) | (b & ((1<<7) - 1)) as u32;
                        if us > D::MAX {
                            info!("Delay too long");
                            return Err(());
                        }
                        if b & (1<<7) == 0 {
                            break;
                        }
                    }
                    delay.delay_us(us).await;
                }
                byte => {
                    let resource = byte & ((1 << 6) - 1);
                    let command = byte >> 6;
                    self.command(resource, command, &mut request, response).await?;
                }
            }
        }

        Ok(())
    }
}

macro_rules! viking{
    (
        $mod_name:ident {
            $(
                $resource_name:ident ($resource_id:literal) {
                    $($mode_name:ident ($mode_id:literal) : $mode_ty:ty,)*
                }
            )*
        }
    ) => {
        pub mod $mod_name {

            use super::*;

            $(
                #[allow(non_camel_case_types)]
                pub enum $resource_name {
                    $($mode_name($mode_ty)),*
                }

                impl $resource_name {
                    pub fn deinit(self) {
                        use $crate::viking::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.deinit(),)*
                        }
                    }
                }
        
            )*

            pub struct State {
                $(
                    $resource_name: Option<$resource_name>,
                )*
            }

            impl $crate::viking::Resources for State {
                const DESCRIPTOR: &'static [u8] = const {
                    use ::viking_protocol::descriptor::*;
                    const PARTS: &[(u8, Option<u16>, &[u8])] = &[
                        (DESCRIPTOR_TYPE_VIKING, Some(0), &[]),
                        $(
                            (DESCRIPTOR_TYPE_RESOURCE, None, &[]),
                            (DESCRIPTOR_TYPE_IDENTIFIER, None, stringify!($resource_name).as_bytes()),
                            $(
                                (DESCRIPTOR_TYPE_MODE,
                                    Some(<$mode_ty as $crate::viking::ResourceMode>::PROTOCOL),
                                    <$mode_ty as $crate::viking::ResourceMode>::DESCRIPTOR
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

                fn new() -> Self {
                    Self {
                        $(
                            $resource_name: None,
                        )*
                    }
                }

                fn configure(&mut self, resource: u8, mode: u8, config: &[u8]) -> Result<(), ()> {
                    match resource {
                        $(
                            $resource_id => {
                                if let Some(r) = self.$resource_name.take() { r.deinit() }
                                self.$resource_name = Some(match mode {
                                    $($mode_id => $resource_name::$mode_name(<$mode_ty as $crate::viking::ResourceMode>::init(config)?),)*
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

                async fn command(&self, resource: u8, command: u8, buf: &mut &[u8], response: &mut $crate::viking::Writer<'_>) -> Result<(), ()> {
                    use $crate::viking::ResourceMode;
                    match resource {
                        $(
                            $resource_id => match &self.$resource_name {
                                $(Some($resource_name::$mode_name(s)) => s.command(command, buf, response).await,)*
                                _ => Err(())
                            }
                        )*
                        _ => Err(())
                    }
                }

                fn poll_all(&self, waker: &core::task::Waker, buf: &mut $crate::viking::Writer) {
                    use $crate::viking::ResourceMode;
                    $(
                        match &self.$resource_name {
                            $(Some($resource_name::$mode_name(s)) => s.poll_event(waker, $resource_id, buf),)*
                            _ => {}
                        }
                    )*
                }
            }
        }
    }
}

pub(crate) use viking;


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

pub(crate) use const_bytes;
