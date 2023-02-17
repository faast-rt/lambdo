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

    pub fn read_from_serial(&mut self) {
        let mut data: Vec<u8> = Vec::new();

        let mut buf = [0; 2];
        loop {
            match self.serial.read(&mut buf) {
                Ok(t) => {
                    if t > 0 {
                        println!("Buf {:?}", &buf[..t]);

                        // find char 1c (record separator) in the buffer
                        if let Some(i) = buf.iter().position(|&r| r == 0x1c) {
                            // Split the buffer at the position of the record separator
                            let (data_to_add, _) = buf.split_at(i);

                            // Add the data to the data vector
                            data.extend_from_slice(data_to_add);

                            break;
                        } else {
                            // Add the data to the data vector
                            data.extend_from_slice(&buf[..t]);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                Err(e) => eprintln!("{:?}", e),
            }
        }

        // Print the data
        println!("Data vector: {:?}", data);

        self.serial.flush().unwrap();
    }
}
