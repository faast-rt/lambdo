use anyhow::{anyhow, Result};
use log::{error, info};

use super::model::{RequestMessage, ResponseMessage, StatusMessage};

pub struct ExternalApi {
    serial_path: String,
    serial_baud_rate: u32,
}

impl ExternalApi {
    pub fn new(serial_path: String, serial_baud_rate: u32) -> Self {
        Self {
            serial_path,
            serial_baud_rate,
        }
    }

    pub fn read_from_serial(&mut self) -> Result<RequestMessage> {
        info!("Reading from serial port: {}", self.serial_path);

        // Open the serial port
        let mut serial = serialport::new(&self.serial_path, self.serial_baud_rate)
            .open()
            .map_err(|e| anyhow!("Failed to open serial port: {}", e))?;

        // Create a buffer to hold the data
        let mut buf = [0; 128];

        // Create the final vector to hold the data
        let mut data_received: Vec<u8> = Vec::new();

        let mut find_delimiter = false;

        while !find_delimiter {
            match serial.read(&mut buf) {
                Ok(t) => {
                    if t > 0 {
                        info!("Buffer received {:?}", &buf[..t]);
                        find_delimiter =
                            self.append_data_before_delimiter(&buf, &mut data_received)?;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                Err(e) => error!("{:?}", e),
            }
        }

        info!("Final received data: {:?}", data_received);

        let code_entry = self.parse_json_payload(&data_received)?;

        // Flush the serial port
        serial
            .flush()
            .map_err(|e| anyhow!("Failed to flush serial port: {}", e))?;

        Ok(code_entry)
    }

    pub fn append_data_before_delimiter(
        &mut self,
        buf: &[u8],
        data_received: &mut Vec<u8>,
    ) -> Result<bool> {
        // find char 1c (record separator) in the buffer
        if let Some(i) = buf.iter().position(|&r| r == 0x1c) {
            // Split the buffer at the position of the record separator
            let (data_to_add, _) = buf.split_at(i);

            // Add the data to the data vector
            data_received.extend_from_slice(data_to_add);

            info!("Delimiter found at index: {}", i);

            Ok(true)
        } else {
            // Add the data to the data vector
            data_received.extend_from_slice(&buf[..buf.len()]);
            Ok(false)
        }
    }

    pub fn parse_json_payload(&mut self, data: &[u8]) -> Result<RequestMessage> {
        // Convert the data vector to a codeEntry struct
        let request_message: RequestMessage = serde_json::from_slice(data)
            .map_err(|e| anyhow!("Failed to parse JSON payload: {}", e))?;

        info!("Code request message: {:?}", request_message);

        Ok(request_message)
    }

    pub fn send_status_message(&mut self) -> Result<()> {
        let status_message: StatusMessage = StatusMessage::new();
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
        // Open the serial port
        let mut serial = serialport::new(&self.serial_path, self.serial_baud_rate)
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

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use crate::external_api::model::{Code, FileModel, RequestStep, Type};

    use super::ExternalApi;

    #[test]
    fn test_parse_json_payload() -> Result<()> {
        let mut internal_api = ExternalApi::new("".to_string(), 0);

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

        let request_message = internal_api.parse_json_payload(&data)?;

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
        let mut internal_api = ExternalApi::new("".to_string(), 0);

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

        let code_entry = internal_api.parse_json_payload(&data);

        assert!(code_entry.is_err());

        Ok(())
    }

    #[test]
    fn test_data_cut_before_delimiter() -> Result<()> {
        let mut internal_api = ExternalApi::new("".to_string(), 0);

        let data = [97, 98, 99, 28, 1, 2, 3, 4, 5, 6, 7];
        let mut data_received: Vec<u8> = Vec::new();

        let find_demiliter =
            internal_api.append_data_before_delimiter(&data, &mut data_received)?;

        assert!(find_demiliter);
        assert_eq!(data_received, [97, 98, 99]);

        Ok(())
    }

    #[test]
    fn test_data_transferred_without_delimiter() -> Result<()> {
        let mut internal_api = ExternalApi::new("".to_string(), 0);

        let data = [97, 98, 99, 1, 2, 3, 4, 5, 6, 7];
        let mut data_received: Vec<u8> = Vec::new();

        let find_demiliter =
            internal_api.append_data_before_delimiter(&data, &mut data_received)?;

        assert!(!find_demiliter);
        assert_eq!(data_received, [97, 98, 99, 1, 2, 3, 4, 5, 6, 7]);

        Ok(())
    }

    #[test]
    fn test_data_transferred_multiple_time() -> Result<()> {
        let mut internal_api = ExternalApi::new("".to_string(), 0);

        let data = [97, 98, 99];
        let data2 = [1, 2, 3, 4, 5, 6, 7];
        let mut data_received: Vec<u8> = Vec::new();

        let find_demiliter =
            internal_api.append_data_before_delimiter(&data, &mut data_received)?;
        let find_demiliter2 =
            internal_api.append_data_before_delimiter(&data2, &mut data_received)?;

        assert!(!find_demiliter);
        assert!(!find_demiliter2);
        assert_eq!(data_received, [97, 98, 99, 1, 2, 3, 4, 5, 6, 7]);

        Ok(())
    }

    #[test]
    fn test_data_transferred_with_delimiter() -> Result<()> {
        let mut internal_api = ExternalApi::new("".to_string(), 0);

        let data = [97, 98, 99];
        let data2 = [1, 2, 3, 4, 5, 6, 7];
        let data3 = [8, 9, 10, 28, 11, 12, 13];
        let mut data_received: Vec<u8> = Vec::new();

        let find_demiliter =
            internal_api.append_data_before_delimiter(&data, &mut data_received)?;
        let find_demiliter2 =
            internal_api.append_data_before_delimiter(&data2, &mut data_received)?;
        let find_demiliter3 =
            internal_api.append_data_before_delimiter(&data3, &mut data_received)?;

        assert!(!find_demiliter);
        assert!(!find_demiliter2);
        assert!(find_demiliter3);
        assert_eq!(data_received, [97, 98, 99, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        Ok(())
    }
}
