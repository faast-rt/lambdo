// This message is sent to the API server to indicate wether 
// the agent is ready or not to receive messages.

pub const MESSAGE_SIZE_NB_BYTES: usize = 8; 

pub struct Message {
    pub message_size: [u8; MESSAGE_SIZE_NB_BYTES], // These are characters e.g. 00002048
    pub message: Vec<u8> // stringified json, vec because size is unknown
}

impl Message {
    pub fn new(message_to_send: String) -> Self {
        let mut message_size = [0; MESSAGE_SIZE_NB_BYTES];
        let message = message_to_send.as_bytes().to_vec();
        
        let string_size = format!("{:0>8}", message.len());
        //We can't call directly as bytes as both &str and String sizes are not known at
        //compile time unlike message_size 
        
        for (i, c) in string_size.chars().enumerate() {
            message_size[i] = c as u8;
        }

        Self {
            message_size,
            message
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.message_size);
        bytes.extend_from_slice(&self.message);
        bytes
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_message_well_encoded() {
        let message_data = "Hello world".to_string();
        let message = Message::new(message_data);
        assert_eq!(message.message, [72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]);
        assert_eq!(message.message_size, [48, 48, 48, 48, 48, 48, 49, 49]);

        assert_eq!(message.to_bytes(), [48, 48, 48, 48, 48, 48, 49, 49, 72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]);
    }


    #[test]
    fn message_size_badly_encoded() {
        let message_data = "Hello world".to_string();
        let message = Message::new(message_data);
        assert_eq!(message.message, [72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]);
        assert_ne!(message.message_size, [48, 48, 48, 48, 48, 48, 49, 50]); // should be 11, is 12
    }

}
 