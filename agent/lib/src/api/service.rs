use std::net::IpAddr;

use anyhow::{anyhow, Result};
use log::{debug, error, info, trace};

use serialport::SerialPort;

use super::comms::{Message, MESSAGE_SIZE_NB_BYTES};
use shared::{Code, ErrorMessage, RequestMessage, ResponseMessage, StatusMessage};

pub struct Api {
    serial_path: String,

    serial_port: Box<dyn SerialPort>, // So we don't open it multiple times
}

impl Api {
    pub async fn new(serial_path: String, serial_baud_rate: u32, gateway: IpAddr) -> Self {
        Self {
            serial_path: serial_path.clone(),
            serial_port: serialport::new(serial_path, serial_baud_rate)
                .open()
                .unwrap(),
        }
    }

    pub fn read_from_serial(&mut self) -> Result<RequestMessage> {
        info!("Reading from serial port: {}", self.serial_path);

        // Create a buffer to hold the data
        let mut size_buffer = [0u8; MESSAGE_SIZE_NB_BYTES];

        let mut buf = [0; 128];
        let mut bytes_read: usize = 0;

        // Create the final vector to hold the data
        let mut data_received: Vec<u8> = Vec::new();

        //we read the buffer and retrieve the first 8 bytes which are the size of the message
        while bytes_read < MESSAGE_SIZE_NB_BYTES {
            match self.serial_port.read(&mut size_buffer) {
                Ok(t) => {
                    if t > 0 {
                        bytes_read += t;
                        data_received.extend_from_slice(&size_buffer[..t]);
                        debug!("Received {} bytes", t);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                Err(e) => error!("{:?}", e),
            }
        }

        let size_string = String::from_utf8(data_received.clone())
            .map_err(|e| anyhow!("Failed to get message size as string: {}", e))?;

        trace!("Size string: {}", size_string);

        let data_size = size_string
            .parse::<usize>()
            .map_err(|e| anyhow!("Failed to parse length of message: {}", e))?;

        // We clean up the vector to only keep the message
        data_received.drain(..MESSAGE_SIZE_NB_BYTES);

        bytes_read = 0;

        while bytes_read < data_size {
            match self.serial_port.read(&mut buf) {
                Ok(t) => {
                    if t > 0 {
                        bytes_read += t;
                        data_received.extend_from_slice(&buf[..t]);
                        debug!("Received {} bytes", t);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                Err(e) => error!("{:?}", e),
            }
        }

        debug!("Final received data: {:?}", data_received);
        debug!("Total bytes read {:?}", bytes_read);

        let code_entry = self.parse_json_payload(&data_received)?;

        info!("Code entry: {:?}", code_entry);

        Ok(code_entry)
    }

    pub fn parse_json_payload(&mut self, data: &[u8]) -> Result<RequestMessage> {
        // Convert the data vector to a RequestMessage struct
        let request_message: RequestMessage = serde_json::from_slice(data)
            .map_err(|e| anyhow!("Failed to parse JSON payload: {}", e))?;

        info!("Code request message: {:?}", request_message);

        Ok(request_message)
    }

    pub async fn send_status_message(&mut self) -> Result<()> {
        let status_message: StatusMessage = StatusMessage::new(Code::Ready);
        let status_message_json = serde_json::to_string(&status_message)
            .map_err(|e| anyhow!("Failed to serialize status message: {}", e))?;
        self.write_to_serial(&status_message_json)?;
        Ok(())
    }

    pub async fn send_response_message(&mut self, response_message: ResponseMessage) -> Result<()> {
        let code_json = serde_json::to_string(&response_message)
            .map_err(|e| anyhow!("Failed to stringify response message : {}", e))?;

        // Write the JSON to the serial port
        self.write_to_serial(&code_json)?;

        info!(
            "Response message written to serial port: {:?}",
            response_message
        );
        Ok(())
    }

    pub async fn send_error_message(&mut self, error_message: String) -> Result<()> {
        let error = ErrorMessage::new(error_message);
        let error_json = serde_json::to_string(&error)
            .map_err(|e| anyhow!("Failed to stringify error message : {}", e))?;

        // Write the JSON to the serial port
        self.write_to_serial(&error_json)?;

        info!("Error message written to serial port: {:?}", error);
        Ok(())
    }

    pub fn write_to_serial(&mut self, data: &str) -> Result<()> {
        info!("Writing to serial port: {}", self.serial_path);

        // Convert the string to a byte array
        let message = Message::new(data.to_string()).to_bytes();
        let buf = message.as_slice();

        // Write the byte array to the serial port
        self.serial_port
            .write_all(buf)
            .map_err(|e| anyhow!("Failed to write to serial port: {}", e))?;

        // In order to still be readable by ``readline`` on the api side, we add a carriage return
        // (not included in the message size)
        self.serial_port
            .write("\r\n".as_bytes())
            .map_err(|e| anyhow!("Failed to write to serial port: {}", e))?;

        Ok(())
    }
}
