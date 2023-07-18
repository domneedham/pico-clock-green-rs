use ds323x::Ds323x;
use embassy_rp::{i2c, peripherals::I2C1};

pub struct RTC<'a>(
    pub  Ds323x<
        ds323x::interface::I2cInterface<i2c::I2c<'a, I2C1, i2c::Blocking>>,
        ds323x::ic::DS3231,
    >,
);
