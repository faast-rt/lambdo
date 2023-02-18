

use anyhow::{anyhow, Result};
use log::{error, info};

use super::model::CodeEntry;

pub struct ExternalApi {
    serial_read_path: String,
    serial_write_path: String,
    serial_baud_rate: u32,
    data_received: Vec<u8>,
}

impl ExternalApi {
    pub fn new(serial_read_path: String, serial_write_path: String, serial_baud_rate: u32) -> Self {
        let data_received: Vec<u8> = Vec::new();
        Self {
            serial_read_path,
            serial_write_path,
            serial_baud_rate,
            data_received,
        }
    }

    pub fn read_from_serial(&mut self) -> Result<CodeEntry> {
        info!("Reading from serial port: {}", self.serial_read_path);

        // Open the serial port
        let mut serial = serialport::new(&self.serial_read_path, self.serial_baud_rate)
            .open()
            .map_err(|e| anyhow!("Failed to open serial port: {}", e))?;

        // Create a buffer to hold the data
        let mut buf = [0; 128];
        loop {
            match serial.read(&mut buf) {
                Ok(t) => {
                    if t > 0 {
                        info!("Buffer received {:?}", &buf[..t]);

                        // find char 1c (record separator) in the buffer
                        if let Some(i) = buf.iter().position(|&r| r == 0x1c) {
                            // Split the buffer at the position of the record separator
                            let (data_to_add, _) = buf.split_at(i);

                            // Add the data to the data vector
                            self.data_received.extend_from_slice(data_to_add);

                            info!("Delimiter found at index: {}", i);

                            break;
                        } else {
                            // Add the data to the data vector
                            self.data_received.extend_from_slice(&buf[..t]);
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                Err(e) => error!("{:?}", e),
            }
        }

        info!("Final received data: {:?}", self.data_received);
  
        let code_entry = self.parse_json_payload()?;

        // Flush the serial port
        serial
            .flush()
            .map_err(|e| anyhow!("Failed to flush serial port: {}", e))?;

        Ok(code_entry)
    }

    pub fn parse_json_payload(&mut self) -> Result<CodeEntry> {
        
        // Convert the data vector to a codeEntry struct
        let code_entry: CodeEntry = serde_json::from_slice(&self.data_received)
            .map_err(|e| anyhow!("Failed to parse JSON payload: {}", e))?;

        info!("Code entry: {:?}", code_entry);

        // Clear the data vector
        self.data_received = Vec::new();
        
        Ok(code_entry)
    }

    pub fn write_to_serial(&mut self, data: &str) -> Result<()> {
        // Open the serial port
        let mut serial = serialport::new(&self.serial_write_path, self.serial_baud_rate)
            .open()
            .map_err(|e| anyhow!("Failed to open serial port: {}", e))?;

        // Conver the string to a byte array
        let buf = data.as_bytes();

        // Write the byte array to the serial port
        serial
            .write_all(buf)
            .map_err(|e| anyhow!("Failed to write to serial port: {}", e))?;

        // Flush the serial port
        serial
            .flush()
            .map_err(|e| anyhow!("Failed to flush serial port: {}", e))?;
        Ok(())
    }
}
