mod pin;
pub use pin::{alternate, Alternate, AlternateFunc};
mod sercom;

mod gpio;
pub use gpio::{Gpio, LevelInterrupt};

mod i2c;
pub use i2c::{SercomI2C, SercomSCLPin, SercomSDAPin};

mod spi;
pub use spi::{SercomSPI, SercomSCKPin, SercomSOPin, SercomSIPin};