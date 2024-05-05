mod pin;
mod sercom;

mod gpio;
pub use gpio::Gpio;

mod i2c;
pub use i2c::{SercomI2C, SercomSCLPin, SercomSDAPin};

mod spi;
pub use spi::{SercomSPI, SercomSCKPin, SercomSOPin, SercomSIPin};