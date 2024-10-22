pub const PRODUCT_STRING: &'static str = "RP2040 Pico";

pub fn serial_number() -> [u8; 8] {
    [0; 8]
}

pub fn init() {

}

crate::viking::viking!(
    viking_impl {

    }
);
