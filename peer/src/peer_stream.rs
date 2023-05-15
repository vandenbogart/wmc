use std::{net::{SocketAddr, ToSocketAddrs, TcpStream}, io::{Write, Read}, time::Duration};

use byteorder::{BigEndian, ByteOrder};
use anyhow::Context;
use crate::messages::{HandShake, PeerMessage, RawMessage};

#[derive(thiserror::Error, Debug)]
pub enum PeerError {
    #[error("Peer protocol mismatch")]
    BadProtocol,
    #[error("Peer info hash mismatch")]
    BadInfoHash,
}
struct PeerStreamOpts {
    protocol: Vec<u8>,
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
}

struct PeerStream {
    addr: SocketAddr,
    stream: TcpStream,
    handshake: HandShake,
}
impl PeerStream {
    pub fn connect(addr: SocketAddr, opts: PeerStreamOpts) -> anyhow::Result<PeerStream> {
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).context("Failed to connect to peer")?;
        let request_handshake = HandShake {
            pstr: opts.protocol,
            info_hash: opts.info_hash,
            peer_id: opts.peer_id,
        };
        stream.write_all(&request_handshake.to_bytes()).context("Failed to write handshake")?;
        let mut bytes = vec![0u8; request_handshake.to_bytes().len()];
        stream.read_exact(&mut bytes).context("Failed to read handshake")?;
        let response_handshake = HandShake::from_bytes(&bytes);
        if request_handshake.pstr != response_handshake.pstr {
            return Err(PeerError::BadProtocol)?;
        }
        else if request_handshake.info_hash != response_handshake.info_hash {
            return Err(PeerError::BadInfoHash)?;
        }
        Ok(PeerStream {
            addr,
            stream,
            handshake: response_handshake,
        })
    }
    pub fn read_message(&mut self) -> anyhow::Result<RawMessage> {
        let mut length = vec![0u8; 4];
        self.stream.read_exact(&mut length).context("Failed to read message length")?;
        let length = BigEndian::read_int(&length, 4) as usize;
        let mut message_bytes = vec![0u8; length];
        self.stream.read_exact(&mut message_bytes).context("Failed to read message")?;
        Ok(RawMessage::from(&message_bytes[..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_peerstream_connect() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let handshake = HandShake {
                pstr: "test_protocol".as_bytes().to_vec(),
                info_hash: vec![1u8; 20],
                peer_id: vec![2u8; 20],
            };
            stream.write_all(&handshake.to_bytes()).unwrap();
        });
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let peerstream = PeerStream::connect(addr, opts).unwrap();
        assert_eq!(peerstream.handshake.pstr, "test_protocol".as_bytes());
        handle.join().unwrap();
    }

    #[test]
    fn test_peerstream_bad_info_hash() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let handshake = HandShake {
                pstr: "test_protocol".as_bytes().to_vec(),
                info_hash: vec![0u8; 20],
                peer_id: vec![2u8; 20],
            };
            stream.write_all(&handshake.to_bytes()).unwrap();
        });
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let result = PeerStream::connect(addr, opts);
        assert!(result.is_err());
        handle.join().unwrap();
    }

    #[test]
    fn test_peerstream_bad_protocol() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let handshake = HandShake {
                pstr: "fail_protocol".as_bytes().to_vec(),
                info_hash: vec![1u8; 20],
                peer_id: vec![2u8; 20],
            };
            stream.write_all(&handshake.to_bytes()).unwrap();
        });
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let result = PeerStream::connect(addr, opts);
        assert!(result.is_err());
        handle.join().unwrap();
    }

    #[test]
    fn test_peerstream_read_message() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let handshake = HandShake {
                pstr: "test_protocol".as_bytes().to_vec(),
                info_hash: vec![1u8; 20],
                peer_id: vec![2u8; 20],
            };
            stream.write_all(&handshake.to_bytes()).unwrap();
            let message_bytes = vec![0, 0, 0, 5, 1, 1, 2, 3, 4];
            stream.write_all(&message_bytes).unwrap();
        });
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let mut peerstream = PeerStream::connect(addr, opts).unwrap();
        let message = peerstream.read_message().unwrap();
        assert_eq!(message.message_id, 1);
        assert_eq!(message.payload, vec![1, 2, 3, 4]);
        handle.join().unwrap();
    }

    #[test]
    fn test_peerstream_read_message_keep_alive() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let handshake = HandShake {
                pstr: "test_protocol".as_bytes().to_vec(),
                info_hash: vec![1u8; 20],
                peer_id: vec![2u8; 20],
            };
            stream.write_all(&handshake.to_bytes()).unwrap();
            let message_bytes = vec![0, 0, 0, 0, 0];
            stream.write_all(&message_bytes).unwrap();
        });
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let mut peerstream = PeerStream::connect(addr, opts).unwrap();
        let message = peerstream.read_message().unwrap();
        assert_eq!(message.message_id, 0);
        assert_eq!(message.payload, vec![]);
        handle.join().unwrap();
    }
}
