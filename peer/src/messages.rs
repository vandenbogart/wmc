use byteorder::{BigEndian, ByteOrder};


pub trait PeerMessage {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}

#[derive(Debug, PartialEq)]
pub struct HandShake {
    pub pstr: Vec<u8>,
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
}
impl PeerMessage for HandShake {
    fn to_bytes(&self) -> Vec<u8> {
        let pstrlen = self.pstr.len();
        let size = 49 + pstrlen;
        let mut bytes = vec![0u8; size];
        // pstrlen
        BigEndian::write_int(&mut bytes, pstrlen as i64, 1);
        // pstr
        let end_pstr = pstrlen + 1;
        bytes[1..end_pstr].copy_from_slice(&self.pstr);
        // reserved
        let end_reserved = end_pstr + 8;
        let reserved = vec![0u8; 8];
        bytes[end_pstr..end_reserved].copy_from_slice(&reserved);
        // info hash
        let end_info_hash = end_reserved + 20;
        bytes[end_reserved..end_info_hash].copy_from_slice(&self.info_hash);
        // peer id
        let end_peer_id = end_info_hash + 20;
        bytes[end_info_hash..end_peer_id].copy_from_slice(&self.peer_id);
        bytes
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        // pstrlen
        let pstrlen = BigEndian::read_int(bytes, 1) as usize;
        let end_pstr = pstrlen + 1;
        // pstr
        let pstr = bytes[1..end_pstr].to_vec();
        // reserved
        let end_reserved = end_pstr + 8;
        bytes[end_pstr..end_reserved].to_vec();
        // info hash
        let end_info_hash = end_reserved + 20;
        let info_hash = bytes[end_reserved..end_info_hash].to_vec();
        // peer id
        let end_peer_id = end_info_hash + 20;
        let peer_id = bytes[end_info_hash..end_peer_id].to_vec();
        Self {
            pstr,
            info_hash,
            peer_id,
        }
    }
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
impl From<RawMessage> for Vec<u8> {
    fn from(raw_message: RawMessage) -> Self {
        let mut bytes = vec![0u8; raw_message.payload.len() + 1];
        bytes[0] = raw_message.message_id;
        bytes[1..].copy_from_slice(&raw_message.payload);
        bytes
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

    #[test]
    fn test_raw_message_into_bytes() {
        let raw_message = RawMessage {
            message_id: 5,
            payload: vec![1,2,3,4,5],
        };
        let expected_bytes: Vec<u8> = vec![5, 1, 2, 3, 4, 5];
        let bytes: Vec<u8> = raw_message.into();
        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn test_empty_raw_message_from_bytes() {
        let input: Vec<u8> = vec![];
        let raw_message = RawMessage::from(&input[..]);
        assert_eq!(raw_message.message_id, 0);
        let expected_payload: Vec<u8> = vec![];
        assert_eq!(raw_message.payload, expected_payload);
    }

    #[test]
    fn test_empty_raw_message_into_bytes() {
        let raw_message = RawMessage {
            message_id: 0,
            payload: vec![],
        };
        let expected_bytes: Vec<u8> = vec![0];
        let bytes: Vec<u8> = raw_message.into();
        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn test_handshake_conversions() {
        let mut pstr = vec![0u8; 10];
        pstr.copy_from_slice("protocol88".as_bytes());
        let mut info_hash = vec![0u8; 20];
        info_hash.copy_from_slice("abcdefghijklmnopijuo".as_bytes());
        let mut peer_id = vec![0u8; 20];
        peer_id.copy_from_slice("abcdefghijklmnopijll".as_bytes());
        let handshake = HandShake {
            pstr,
            info_hash,
            peer_id,
        };

        let bytes: Vec<u8> = handshake.to_bytes();
        let new_handshake = HandShake::from_bytes(&bytes);
        assert_eq!(handshake, new_handshake);
    }
    

}
