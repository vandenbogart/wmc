use std::{
    cmp::{max, min},
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    time::Duration,
};

use crate::messages::{HandShake, PeerMessage, RawMessage};
use anyhow::Context;
use byteorder::{BigEndian, ByteOrder};

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
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
            .context("Failed to connect to peer")?;
        let response_handshake = PeerStream::handshake(&stream, opts)?;
        Ok(PeerStream {
            addr,
            stream,
            handshake: response_handshake,
        })
    }
    fn handshake(
        ref mut stream: impl Read + Write + Unpin,
        opts: PeerStreamOpts,
    ) -> anyhow::Result<HandShake> {
        let request_handshake = HandShake {
            pstr: opts.protocol,
            info_hash: opts.info_hash,
            peer_id: opts.peer_id,
        };
        stream
            .write_all(&request_handshake.to_bytes())
            .context("Failed to write handshake")?;
        let mut bytes = vec![0u8; request_handshake.to_bytes().len()];
        stream
            .read_exact(&mut bytes)
            .context("Failed to read handshake")?;
        let response_handshake = HandShake::from_bytes(&bytes);
        if request_handshake.pstr != response_handshake.pstr {
            return Err(PeerError::BadProtocol)?;
        } else if request_handshake.info_hash != response_handshake.info_hash {
            return Err(PeerError::BadInfoHash)?;
        }
        Ok(response_handshake)
    }
    fn read_message(ref mut stream: impl Read + Write + Unpin) -> anyhow::Result<RawMessage> {
        let mut length = vec![0u8; 4];
        stream
            .read_exact(&mut length)
            .context("Failed to read message length")?;
        let length = BigEndian::read_int(&length, 4) as usize;
        let mut message_bytes = vec![0u8; length];
        stream
            .read_exact(&mut message_bytes)
            .context("Failed to read message")?;
        Ok(RawMessage::from(&message_bytes[..]))
    }
}

struct MockTcpStream {
    read_data: Vec<u8>,
    write_data: Vec<u8>,
}
impl Read for MockTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let end = min(buf.len(), self.read_data.len());
        buf[..end].copy_from_slice(&self.read_data[..end]);
        self.read_data = self.read_data[end..].to_vec();
        Ok(end)
    }
}
impl Write for MockTcpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_data = Vec::from(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Unpin for MockTcpStream {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_peerstream_handshake() {
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let expected_response = HandShake {
            pstr: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let mut stream = MockTcpStream {
            read_data: expected_response.to_bytes().to_vec(),
            write_data: Vec::new(),
        };
        let response = PeerStream::handshake(&mut stream, opts).unwrap();
        assert_eq!(response.pstr, "test_protocol".as_bytes());
    }

    #[test]
    fn test_peerstream_bad_info_hash() {
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![0u8; 20],
            peer_id: vec![2u8; 20],
        };
        let expected_response = HandShake {
            pstr: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let mut stream = MockTcpStream {
            read_data: expected_response.to_bytes().to_vec(),
            write_data: Vec::new(),
        };
        let response = PeerStream::handshake(&mut stream, opts);
        assert!(response.is_err());
        assert_eq!(
            response.err().unwrap().to_string(),
            "Peer info hash mismatch"
        );
    }

    #[test]
    fn test_peerstream_bad_protocol() {
        let opts = PeerStreamOpts {
            protocol: "test_protocol".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![0u8; 20],
        };
        let expected_response = HandShake {
            pstr: "test_protocok".as_bytes().to_vec(),
            info_hash: vec![1u8; 20],
            peer_id: vec![2u8; 20],
        };
        let mut stream = MockTcpStream {
            read_data: expected_response.to_bytes().to_vec(),
            write_data: Vec::new(),
        };
        let response = PeerStream::handshake(&mut stream, opts);
        assert!(response.is_err());
        assert_eq!(
            response.err().unwrap().to_string(),
            "Peer protocol mismatch"
        );
    }

    #[test]
    fn test_peerstream_read_message() {
        let mut stream = MockTcpStream {
            read_data: vec![0, 0, 0, 4, 1, 2, 2, 4],
            write_data: Vec::new(),
        };
        let response = PeerStream::read_message(&mut stream).unwrap();
        assert_eq!(response.message_id, 1);
        assert_eq!(response.payload, vec![2, 2, 4]);
    }

    #[test]
    fn test_peerstream_read_message_keep_alive() {
        let mut stream = MockTcpStream {
            read_data: vec![0, 0, 0, 0],
            write_data: Vec::new(),
        };
        let response = PeerStream::read_message(&mut stream).unwrap();
        assert_eq!(response.message_id, 0);
        assert_eq!(response.payload, vec![]);
    }
}
