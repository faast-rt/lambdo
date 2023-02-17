use serialport::{Error, SerialPort};

pub struct ExternalApi {
    serial_read_path: String,
    serial_write_path: String,
    serial_baud_rate: u32,
}

impl ExternalApi {
    pub fn new(serial_read_path: String, serial_write_path: String, serial_baud_rate: u32) -> Self {
        Self {
            serial_read_path,
            serial_write_path,
            serial_baud_rate,
        }
    }

    pub fn read_from_serial(&mut self) {
        // Open the serial port
        let mut serial = serialport::new(&self.serial_read_path, self.serial_baud_rate)
            .open()
            .unwrap();

        // Create a vector to hold the data
        let mut data: Vec<u8> = Vec::new();

        // Create a buffer to hold the data
        let mut buf = [0; 128];
        loop {
            match serial.read(&mut buf) {
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

        serial.flush().unwrap();
    }

    pub fn write_to_serial(data: &str) {
        // Open the serial port
        let mut serial = serialport::new(self.serial_write_path, self.serial_baud_rate)
            .open()
            .unwrap();

        // Conver the string to a byte array
        let buf = data.as_bytes();

        // Write the byte array to the serial port
        serial.write_all(buf).unwrap();

        // Flush the serial port
        serial.flush().unwrap();
    }
}
