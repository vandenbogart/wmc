use byteorder::{BigEndian, ByteOrder};


pub trait PeerMessage {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(&self, bytes: &[u8]) -> Self;
}

#[repr(u8)]
pub enum MessageTypes {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
    Port = 9,
}
impl From<u8> for MessageTypes {
    fn from(value: u8) -> Self {
        match value {
            1 => MessageTypes::Unchoke,
            2 => MessageTypes::Interested,
            3 => MessageTypes::NotInterested,
            4 => MessageTypes::Have,
            5 => MessageTypes::Bitfield,
            6 => MessageTypes::Request,
            7 => MessageTypes::Piece,
            8 => MessageTypes::Cancel,
            9 => MessageTypes::Port,
            _ => panic!("Invalid value for message type"),
        }
    }
}

struct RawMessage {
    message_id: u8,
    payload: Vec<u8>,
}
impl From<&[u8]> for RawMessage {
    fn from(bytes: &[u8]) -> Self {
        if bytes.len() == 0 {
            return Self {
                message_id: 0,
                payload: Vec::new(),
            };
        }
        let payload_length = bytes.len() - 1 as usize;
        let message_id = BigEndian::read_int(&bytes, 1) as u8;
        let mut payload = vec![0u8; payload_length];
        payload.copy_from_slice(&bytes[1..]);
        Self {
            message_id,
            payload,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_message_from_bytes() {
        let input: Vec<u8> = vec![5, 1, 2, 3, 4, 5];
        let raw_message = RawMessage::from(&input[..]);
        assert_eq!(raw_message.message_id, 5);
        let expected_payload: Vec<u8> = vec![1,2,3,4,5];
        assert_eq!(raw_message.payload, expected_payload);
    }

    // Bitfield
    // #[test]
    // fn test_bitfield_read() {
    //     let input: [u8] = [

    // }
    // fn test_bitfield_write() {

    // }

}
