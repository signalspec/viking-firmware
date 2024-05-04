use core::mem;

use viking_protocol::AsBytes;

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
    fn describe() -> &'static [u8];

    fn init(config: &[u8]) -> Result<Self, ()>;

    fn deinit(self);

    async fn command(&self, cmd: u8, buf: &mut &[u8], res: &mut Writer<'_>) -> Result<(), ()> {
        Err(())
    }

    fn poll_event(&self, resource: u8, buf: &mut Writer<'_>) {}
}

pub trait Resources: Sized {
    const RESOURCE_NAMES: &'static str;
    fn mode_names(resource: u8) -> Option<&'static str>;
    fn describe(resource: u8, mode: u8) -> Option<&'static [u8]>;

    fn new() -> Self;
    async fn configure(&mut self, resource: u8, mode: u8, config: &[u8]) -> Result<(), ()> ;
    async fn command(&self, resource: u8, command: u8, buf: &mut &[u8], response: &mut Writer) -> Result<(), ()> ;
    fn poll_all(&self, buf: &mut Writer<'_>);

    async fn run(&self, request: &[u8], response: &mut Writer<'_>) -> Result<(), ()> {
        let mut request = request;

        while let Some((byte, mut remaining)) = request.split_first() {
            let resource = byte & ((1 << 6) - 1);
            let command = byte >> 6;

            self.command(resource, command, &mut remaining, response).await?;
            
            request = remaining;
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! viking{
    (
        $mod_name:ident {
            use {$( $use_tt:tt )*};

            $(
                $resource_name:ident ($resource_id:literal) {
                    $($mode_name:ident ($mode_id:literal) : $mode_ty:ty,)*
                }
            )*
        }
    ) => {
        mod $mod_name {

            use {$( $use_tt )*};

            $(
                #[allow(non_camel_case_types)]
                pub enum $resource_name {
                    $($mode_name($mode_ty))*
                }

                impl $resource_name {
                    pub const MODE_NAMES: &'static str = concat!(
                        $(stringify!($mode_name), "\0"),*
                    );

                    pub fn describe(mode: u8) -> Option<&'static [u8]> {
                        match mode {
                            $($mode_id => Some(<$mode_ty as $crate::viking::ResourceMode>::describe()),)*
                            _ => None,
                        }
                    }

                    pub fn init(mode: u8, config: &[u8]) -> Result<Self, ()> {
                        Ok(match mode {
                            $($mode_id => Self::$mode_name(<$mode_ty as $crate::viking::ResourceMode>::init(config)?),)*
                            _ => return Err(())
                        })
                    }

                    pub fn deinit(self) {
                        use $crate::viking::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.deinit(),)*
                            _ => {}
                        }
                    }

                    pub async fn command(&self, cmd: u8, buf: &mut &[u8], response: &mut $crate::viking::Writer<'_>) -> Result<(), ()> {
                        use $crate::viking::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.command(cmd, buf, response).await,)*
                            _ => Err(())
                        }
                    }

                    pub fn poll_event(&self, resource: u8, buf: &mut $crate::viking::Writer<'_>) {
                        use $crate::viking::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.poll_event(resource, buf),)*
                            _ => {}
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
                const RESOURCE_NAMES: &'static str = concat!(
                    $(stringify!($resource_name), "\0"),*
                );

                fn mode_names(resource: u8) -> Option<&'static str> {
                    match resource {
                        $(
                            $resource_id => Some($resource_name::MODE_NAMES),
                        )*
                        _ => None,
                    }
                }
                
                fn describe(resource: u8, mode: u8) -> Option<&'static [u8]> {
                    match resource {
                        $(
                            $resource_id => $resource_name::describe(mode),
                        )*
                        _ => None,
                    }
                }
                
                fn new() -> Self {
                    Self {
                        $(
                            $resource_name: None,
                        )*
                    }
                }

                async fn configure(&mut self, resource: u8, mode: u8, config: &[u8]) -> Result<(), ()> {
                    match resource {
                        $(
                            $resource_id => {
                                if let Some(r) = self.$resource_name.take() { r.deinit() }
                                self.$resource_name = Some($resource_name::init(mode, config)?);
                                Ok(())
                            }
                        )*
                        _ => Err(()),
                    }
                }

                async fn command(&self, resource: u8, command: u8, buf: &mut &[u8], response: &mut $crate::viking::Writer<'_>) -> Result<(), ()> {
                    match resource {
                        $($resource_id => self.$resource_name
                            .as_ref().ok_or(())?
                            .command(command, buf, response).await,
                        )*
                        _ => Err(())
                    }
                }

                fn poll_all(&self, buf: &mut $crate::viking::Writer) {
                    $(
                        if let Some(r) = self.$resource_name.as_ref() {
                            r.poll_event($resource_id, buf)
                        }
                    )*
                }
            }
        }
    }
}
