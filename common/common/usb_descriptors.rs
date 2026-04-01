use zeptos::usb::descriptors::{descriptors, Device, Config, Interface, Endpoint, BinaryObjectStore, PlatformCapabilityMicrosoftOs, MicrosoftOsCompatibleID, MicrosoftOs};

pub const INTF_VIKING: u8 = 0;

pub const STRING_MFG: u8 = 1;
pub const STRING_PRODUCT: u8 = 2;
pub const STRING_SERIAL: u8 = 3;

pub const EP_OUT: u8 = 0x01;
pub const EP_IN: u8 = 0x82;
pub const EP_EVT: u8 = 0x83;

pub const MANUFACTURER_STRING: &str = "signalspec project";

pub static DEVICE_DESCRIPTOR: &[u8] = descriptors! {
    Device {
        bcdUSB: 0x0201,
        bDeviceClass: usb::class_code::DEVICE,
        bDeviceSubClass: 0x00,
        bDeviceProtocol: 0x00,
        bMaxPacketSize0: 64,
        idVendor: 0x59e3,
        idProduct: 0x2222,
        bcdDevice: 0x0000,
        iManufacturer: STRING_MFG,
        iProduct: STRING_PRODUCT,
        iSerialNumber: STRING_SERIAL,
        bNumConfigurations: 1,
    }
};

pub static CONFIG_DESCRIPTOR: &[u8] = descriptors!{
    Config {
        bConfigurationValue: 1,
        iConfiguration: 0,
        bmAttributes: 0x80,
        bMaxPower: 250,

        +Interface {
            bInterfaceNumber: INTF_VIKING,
            bAlternateSetting: 0,
            bInterfaceClass: 0xff,
            bInterfaceSubClass: 0,
            bInterfaceProtocol: 0,
            iInterface: 0,
        }

        +Interface {
            bInterfaceNumber: INTF_VIKING,
            bAlternateSetting: 1,
            bInterfaceClass: 0xff,
            bInterfaceSubClass: 0,
            bInterfaceProtocol: 0,
            iInterface: 0,

            +Endpoint {
                bEndpointAddress: EP_OUT,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }

            +Endpoint {
                bEndpointAddress: EP_IN,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }

            +Endpoint {
                bEndpointAddress: EP_EVT,
                bmAttributes: 0b10,
                wMaxPacketSize: 64,
                bInterval: 0,
            }
        }
    }
};

pub const MSOS_DESCRIPTOR: &[u8] = descriptors!{
    MicrosoftOs {
        windows_version: 0x06030000,

        +MicrosoftOsCompatibleID {
            compatible_id: "WINUSB",
            sub_compatible_id: "",
        }
    }
};

pub const MSOS_VENDOR_CODE: u8 = 0xf0;

pub static BOS_DESCRIPTOR: &[u8] = descriptors!{
    BinaryObjectStore {
        +PlatformCapabilityMicrosoftOs {
            windows_version: 0x06030000,
            vendor_code: MSOS_VENDOR_CODE,
            alt_enum_code: 0,
            msos_descriptor_len: MSOS_DESCRIPTOR.len(),
        }
    }
};
