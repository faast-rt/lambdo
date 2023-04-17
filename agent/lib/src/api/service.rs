use anyhow::{anyhow, Result};
use log::{debug, error, info, trace};

use serialport::SerialPort;

use super::comms::{Message, MESSAGE_SIZE_NB_BYTES};
use super::model::{Code, RequestMessage, ResponseMessage, StatusMessage};

pub struct Api {
    serial_path: String,

    serial_port: Box<dyn SerialPort>, // So we don't open it multiple times
}

impl Api {
    pub fn new(serial_path: String, serial_baud_rate: u32) -> Self {
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

    pub fn send_status_message(&mut self) -> Result<()> {
        let status_message: StatusMessage = StatusMessage::new(Code::Ready);
        let status_message_json = serde_json::to_string(&status_message)
            .map_err(|e| anyhow!("Failed to serialize status message: {}", e))?;
        self.write_to_serial(&status_message_json)?;
        Ok(())
    }

    pub fn send_response_message(&mut self, response_message: ResponseMessage) -> Result<()> {
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

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use crate::api::model::{Code, FileModel, RequestStep, Type};

    use super::Api;

    #[test]
    fn test_parse_json_payload() -> Result<()> {
        let mut api = Api::new("".to_string(), 0);

        // Data vector with the following JSON payload:
        // {
        //     "type": "Request",
        //     "code": "Run",
        //     "data": {
        //       "id": "4bf68974-c315-4c41-aee2-3dc2920e76e9",
        //       "files": [
        //         {
        //           "filename": "src/index.js",
        //           "content": "console.log('Hello World!');"
        //         }
        //       ],
        //       "steps": [
        //         {
        //           "command": "node src/index.js",
        //           "enable_output": true
        //         }
        //       ]
        //     }
        //   }

        let data = [
            123, 10, 32, 32, 34, 116, 121, 112, 101, 34, 58, 32, 34, 82, 101, 113, 117, 101, 115,
            116, 34, 44, 10, 32, 32, 34, 99, 111, 100, 101, 34, 58, 32, 34, 82, 117, 110, 34, 44,
            10, 32, 32, 34, 100, 97, 116, 97, 34, 58, 32, 123, 10, 32, 32, 32, 32, 34, 105, 100,
            34, 58, 32, 34, 52, 98, 102, 54, 56, 57, 55, 52, 45, 99, 51, 49, 53, 45, 52, 99, 52,
            49, 45, 97, 101, 101, 50, 45, 51, 100, 99, 50, 57, 50, 48, 101, 55, 54, 101, 57, 34,
            44, 10, 32, 32, 32, 32, 34, 102, 105, 108, 101, 115, 34, 58, 32, 91, 10, 32, 32, 32,
            32, 32, 32, 123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 34, 102, 105, 108, 101, 110, 97,
            109, 101, 34, 58, 32, 34, 115, 114, 99, 47, 105, 110, 100, 101, 120, 46, 106, 115, 34,
            44, 10, 32, 32, 32, 32, 32, 32, 32, 32, 34, 99, 111, 110, 116, 101, 110, 116, 34, 58,
            32, 34, 99, 111, 110, 115, 111, 108, 101, 46, 108, 111, 103, 40, 39, 72, 101, 108, 108,
            111, 32, 87, 111, 114, 108, 100, 33, 39, 41, 59, 34, 10, 32, 32, 32, 32, 32, 32, 125,
            10, 32, 32, 32, 32, 93, 44, 10, 32, 32, 32, 32, 34, 115, 116, 101, 112, 115, 34, 58,
            32, 91, 10, 32, 32, 32, 32, 32, 32, 123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 34, 99,
            111, 109, 109, 97, 110, 100, 34, 58, 32, 34, 110, 111, 100, 101, 32, 115, 114, 99, 47,
            105, 110, 100, 101, 120, 46, 106, 115, 34, 44, 10, 32, 32, 32, 32, 32, 32, 32, 32, 34,
            101, 110, 97, 98, 108, 101, 95, 111, 117, 116, 112, 117, 116, 34, 58, 32, 116, 114,
            117, 101, 10, 32, 32, 32, 32, 32, 32, 125, 10, 32, 32, 32, 32, 93, 10, 32, 32, 125, 10,
            125,
        ];

        let request_message = api.parse_json_payload(&data)?;

        let files = vec![FileModel {
            filename: "src/index.js".to_string(),
            content: "console.log('Hello World!');".to_string(),
        }];

        let steps = vec![RequestStep {
            command: "node src/index.js".to_string(),
            enable_output: true,
        }];

        assert_eq!(request_message.r#type, Type::Request);
        assert_eq!(request_message.code, Code::Run);
        assert_eq!(
            request_message.data.id,
            "4bf68974-c315-4c41-aee2-3dc2920e76e9"
        );
        assert_eq!(request_message.data.files[0], files[0]);
        assert_eq!(request_message.data.steps[0], steps[0]);
        Ok(())
    }

    #[test]
    fn test_parse_json_payload_failed() -> Result<()> {
        let mut api = Api::new("".to_string(), 0);

        // Data vector with missing comma
        let data = [
            123, 10, 32, 32, 34, 102, 105, 108, 101, 34, 58, 32, 91, 10, 32, 32, 32, 32, 123, 10,
            32, 32, 32, 32, 32, 32, 34, 102, 105, 108, 101, 110, 97, 109, 101, 34, 58, 32, 34, 116,
            101, 115, 116, 46, 112, 121, 34, 44, 10, 32, 32, 32, 32, 32, 32, 34, 99, 111, 110, 116,
            101, 110, 116, 34, 58, 32, 34, 112, 114, 105, 110, 116, 40, 39, 72, 101, 108, 108, 111,
            32, 87, 111, 114, 108, 100, 39, 41, 34, 10, 32, 32, 32, 32, 125, 10, 32, 32, 93, 44,
            10, 32, 32, 34, 115, 99, 114, 105, 112, 116, 34, 58, 32, 91, 10, 32, 32, 32, 32, 34,
            112, 121, 116, 104, 111, 110, 51, 32, 116, 101, 115, 116, 46, 112, 121, 34, 10, 32, 32,
            93, 10, 32, 32, 10, 125, 10,
        ];

        let code_entry = api.parse_json_payload(&data);

        assert!(code_entry.is_err());

        Ok(())
    }
}
