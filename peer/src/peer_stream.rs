use std::{net::{SocketAddr, ToSocketAddrs, TcpStream}, io::{Write, Read}, time::Duration, error::Error};

use crate::messages::{HandShake, PeerMessage};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum PeerError {
    #[error("Info hash did not match")]
    BadInfoHash,
    #[error("Unsupported protocol")]
    BadProtocol,
    #[error("Failed to connect to peer")]
    ConnectionFailure,
    #[error("Failed to send handshake")]
    HandShakeSendFailed,
    #[error("Failed to complete handshake")]
    HandShakeFailed,

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
    pub fn connect(addr: SocketAddr, opts: PeerStreamOpts) -> Result<PeerStream, PeerError> {
        let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(stream) => stream,
            Err(_) => return Err(PeerError::ConnectionFailure)
        };
        let request_handshake = HandShake {
            pstr: opts.protocol,
            info_hash: opts.info_hash,
            peer_id: opts.peer_id,
        };
        match stream.write_all(&request_handshake.to_bytes()) {
            Ok(_) => (),
            Err(_) => return Err(PeerError::HandShakeSendFailed)
        };
        let mut bytes = vec![0u8; request_handshake.to_bytes().len()];
        match stream.read_exact(&mut bytes) {
            Ok(_) => (),
            Err(_) => return Err(PeerError::HandShakeFailed)
        };
        let response_handshake = HandShake::from_bytes(&bytes);
        if request_handshake.pstr != response_handshake.pstr {
            return Err(PeerError::BadProtocol);
        }
        else if request_handshake.info_hash != response_handshake.info_hash {
            return Err(PeerError::BadInfoHash);
        }
        Ok(PeerStream {
            addr,
            stream,
            handshake: response_handshake,
        })
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
        assert_eq!(result.err().unwrap(), PeerError::BadInfoHash);
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
        assert_eq!(result.err().unwrap(), PeerError::BadProtocol);
        handle.join().unwrap();
    }
}
