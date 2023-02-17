use serialport::{Error, SerialPort};

pub struct ExternalApi {
    serial: Box<dyn SerialPort>,
}

impl ExternalApi {
    pub fn new(path: String, baud_rate: u32) -> Result<Self, Error> {
        Ok(Self {
            serial: serialport::new(path, baud_rate).open()?,
        })
    }
}
