use viking_protocol::AsBytes;
pub trait ResourceMode: Sized {
    fn describe() -> &'static [u8];

    fn init(config: &[u8]) -> Result<Self, ()>;

    fn deinit(self);

    async fn command(&self, cmd: u8, buf: &mut &[u8]) -> Result<(), ()>;

    fn poll_event(&self, resource: u8, buf: &mut [u8]) -> usize {
        0
    }
}

pub trait Resources: Sized {
    const RESOURCE_NAMES: &'static str;
    fn mode_names(resource: u8) -> Option<&'static str>;
    fn describe(resource: u8, mode: u8) -> Option<&'static [u8]>;

    fn new() -> Self;
    async fn configure(&mut self, resource: u8, mode: u8, config: &[u8]) -> Result<(), ()> ;
    async fn command(&self, resource: u8, command: u8, buf: &mut &[u8]) -> Result<(), ()> ;
    fn poll_all(&self, buf: &mut [u8]) -> usize;

    async fn run(&self, request: &[u8]) -> Result<(), ()> {
        let mut request = request;

        while let Some((byte, mut remaining)) = request.split_first() {
            let resource = byte & ((1 << 6) - 1);
            let command = byte >> 6;

            self.command(resource, command, &mut remaining).await?;
            
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

                    pub async fn command(&self, cmd: u8, buf: &mut &[u8]) -> Result<(), ()> {
                        use $crate::viking::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.command(cmd, buf).await,)*
                            _ => Err(())
                        }
                    }

                    pub fn poll_event(&self, resource: u8, buf: &mut [u8]) -> usize {
                        use $crate::viking::ResourceMode;
                        match self {
                            $(Self::$mode_name(s) => s.poll_event(resource, buf),)*
                            _ => 0
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

                async fn command(&self, resource: u8, command: u8, buf: &mut &[u8]) -> Result<(), ()> {
                    match resource {
                        $($resource_id => self.$resource_name
                            .as_ref().ok_or(())?
                            .command(command, buf).await,
                        )*
                        _ => Err(())
                    }
                }

                fn poll_all(&self, mut buf: &mut [u8]) -> usize {
                    let mut n = 0;
                    $(
                        n += if let Some(r) = self.$resource_name.as_ref() { r.poll_event($resource_id, &mut buf[n..]) } else { 0 };
                    )*
                    n
                }
            }
        }
    }
}
