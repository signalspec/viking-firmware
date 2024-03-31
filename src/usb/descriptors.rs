#[macro_export]
macro_rules! descriptors {
    (
        $struct:path {
            $($field:ident: $value:expr,)*

            $(+$child:path { $($inner:tt)* })*
        }
    ) => {{
        use $struct as Desc; // https://github.com/rust-lang/rust/issues/48067
        const CHILDREN: &[&[u8]] = {
            &[
                $(
                    descriptors!($child { $($inner)* })
                ),*
            ]
        };
        const LEN: usize = {
            let mut i = 0;
            let mut len = Desc::LEN;
            while i < CHILDREN.len() {
                len += CHILDREN[i].len();
                i += 1;
            }
            len
        };
        const ARR: [u8; LEN] = {
            let desc = Desc::bytes(Desc {
                $($field: $value),*
            }, CHILDREN);

            assert!(desc.len() == Desc::LEN);

            let mut bytes = [0u8; LEN];
            let mut pos = 0;
            
            while pos < desc.len() {
                bytes[pos] = desc[pos];
                pos += 1;
            }

            let mut child = 0;
            while child < CHILDREN.len() {
                let mut cpos = 0;
                while cpos < CHILDREN[child].len() {
                    bytes[pos] = CHILDREN[child][cpos];
                    cpos += 1;
                    pos += 1;
                }
                child += 1;
            }

            bytes
        };
        
        &ARR
    }}
}

#[allow(non_snake_case)]
pub struct Device {
    pub bcdUSB: u16,
    pub bDeviceClass: u8,
    pub bDeviceSubClass: u8,
    pub bDeviceProtocol: u8,
    pub bMaxPacketSize0: u8,
    pub idVendor: u16,
    pub idProduct: u16,
    pub bcdDevice: u16,
    pub iManufacturer: u8,
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub bNumConfigurations: u8,
}

impl Device {
    pub const LEN: usize = 18;
    pub const DESCRIPTOR_TYPE: u8 = usb::descriptor_type::DEVICE;

    pub const fn bytes(self, children: &[&[u8]]) -> [u8; Self::LEN] {
        assert!(children.is_empty());

        [
            Self::LEN as u8,
            Self::DESCRIPTOR_TYPE,
            self.bcdUSB.to_le_bytes()[0],
            self.bcdUSB.to_le_bytes()[1],
            self.bDeviceClass,
            self.bDeviceSubClass,
            self.bDeviceProtocol,
            self.bMaxPacketSize0,
            self.idVendor.to_le_bytes()[0],
            self.idVendor.to_le_bytes()[1],
            self.idProduct.to_le_bytes()[0],
            self.idProduct.to_le_bytes()[1],
            self.bcdDevice.to_le_bytes()[0],
            self.bcdDevice.to_le_bytes()[1],
            self.iManufacturer,
            self.iProduct,
            self.iSerialNumber,
            self.bNumConfigurations,
        ]


    }
}

#[allow(non_snake_case)]
pub struct Config {
    pub bConfigurationValue: u8,
    pub iConfiguration: u8,
    pub bmAttributes: u8,
    pub bMaxPower: u8,
}

impl Config {
    pub const LEN: usize = 9;
    pub const DESCRIPTOR_TYPE: u8 = usb::descriptor_type::CONFIGURATION;

    pub const fn bytes(self, children: &[&[u8]]) -> [u8; Self::LEN] {
        let mut total_len = Self::LEN as u16;
        let mut interface_number = 0;
        let mut num_interfaces = 0;

        let mut i = 0;
        while i < children.len() {
            total_len += children[i].len() as u16;
            if children[i][1] == Interface::DESCRIPTOR_TYPE {
                let intf = children[i][2];
                assert!(intf == interface_number || intf == interface_number + 1, "interface numbers must be contiguous");
                interface_number = intf;
                num_interfaces = intf + 1;
            }
            i += 1;
        }

        [
            Self::LEN as u8,
            Self::DESCRIPTOR_TYPE,
            total_len.to_le_bytes()[0],
            total_len.to_le_bytes()[1],
            num_interfaces,
            self.bConfigurationValue,
            self.iConfiguration,
            self.bmAttributes,
            self.bMaxPower,
        ]
    }
}

/// Fields of a USB interface descriptor.
///
/// The `bLength` and `bDescriptorType` are fixed. `bNumEndpoints` is populated from the
/// number of child endpoint descriptors.
#[allow(non_snake_case)]
pub struct Interface {
    pub bInterfaceNumber: u8,
    pub bAlternateSetting: u8,
    pub bInterfaceClass: u8,
    pub bInterfaceSubClass: u8,
    pub bInterfaceProtocol: u8,
    pub iInterface: u8,
}

impl Interface {
    pub const LEN: usize = 9;
    pub const DESCRIPTOR_TYPE: u8 = usb::descriptor_type::INTERFACE;

    pub const fn bytes(self, children: &[&[u8]]) -> [u8; Self::LEN] {
        let mut num_endpoints = 0;
        let mut i = 0;
        while i < children.len() {
            if children[i][1] == Endpoint::DESCRIPTOR_TYPE {
                num_endpoints += 1;
            }
            i += 1;
        }

        [
            Self::LEN as u8,
            Self::DESCRIPTOR_TYPE,
            self.bInterfaceNumber,
            self.bAlternateSetting,
            num_endpoints as u8,
            self.bInterfaceClass,
            self.bInterfaceSubClass,
            self.bInterfaceProtocol,
            self.iInterface,
        ]
    }
}

#[allow(non_snake_case)]
pub struct Endpoint {
    pub bEndpointAddress: u8,
    pub bmAttributes: u8,
    pub wMaxPacketSize: u16,
    pub bInterval: u8,
}

impl Endpoint {
    pub const LEN: usize = 7;
    pub const DESCRIPTOR_TYPE: u8 = usb::descriptor_type::ENDPOINT;

    pub const fn bytes(self, _children: &[&[u8]]) -> [u8; Self::LEN] {
        [
            Endpoint::LEN as u8,
            Endpoint::DESCRIPTOR_TYPE,
            self.bEndpointAddress,
            self.bmAttributes,
            self.wMaxPacketSize.to_le_bytes()[0],
            self.wMaxPacketSize.to_le_bytes()[1],
            self.bInterval,
        ]
    }
}

pub struct StringDecriptor<const N: usize>([u8; N]);

impl<const N: usize> StringDecriptor<N> {
    pub fn language_list() -> Self {
        let mut buf = [0; N];
        buf[0] = 4;
        buf[1] = usb::descriptor_type::STRING;
        buf[2] = usb::language_id::ENGLISH_US.to_le_bytes()[0];
        buf[3] = usb::language_id::ENGLISH_US.to_le_bytes()[1];
        Self(buf)
    }

    pub fn new(s: &str) -> Self {
        let mut buf = [0; N];
        let mut len = 2;
        for c in s.encode_utf16() {
            buf[len..len+2].copy_from_slice(&c.to_le_bytes());
            len += 2;
        }
        buf[0] = len as u8;
        buf[1] = usb::descriptor_type::STRING;
        Self(buf)
    }

    pub fn new_hex(s: &[u8]) -> Self {
        let mut buf = [0; N];
        let mut len = 2;

        fn hex(b: u8) -> u8 {
             b"0123456789ABCDEF"[b as usize]
        }

        for b in s.iter() {
            buf[len] = hex(b >> 4);
            buf[len+2] = hex(b & 0x0f);
            len += 4;
        }
        buf[0] = len as u8;
        buf[1] = usb::descriptor_type::STRING;
        Self(buf)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0[..self.0[0] as usize]
    }
}
