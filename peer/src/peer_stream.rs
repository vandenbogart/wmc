use async_std::prelude::*;
use async_std::{
    io::{Read, Write},
    net::TcpStream,
};
use std::{
    cmp::{max, min},
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    task::Poll,
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
    pub async fn read(&mut self) -> anyhow::Result<RawMessage> {
        PeerStream::read_message(&self.stream).await
    }
    pub async fn connect(addr: SocketAddr, opts: PeerStreamOpts) -> anyhow::Result<PeerStream> {
        let stream = TcpStream::connect(&addr)
            .await
            .context("Failed to connect to peer")?;
        let response_handshake = PeerStream::handshake(&stream, opts).await?;
        Ok(PeerStream {
            addr,
            stream,
            handshake: response_handshake,
        })
    }
    async fn handshake(
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
            .await
            .context("Failed to write handshake")?;
        let mut bytes = vec![0u8; request_handshake.to_bytes().len()];
        stream
            .read_exact(&mut bytes)
            .await
            .context("Failed to read handshake")?;
        let response_handshake = HandShake::from_bytes(&bytes);
        if request_handshake.pstr != response_handshake.pstr {
            return Err(PeerError::BadProtocol)?;
        } else if request_handshake.info_hash != response_handshake.info_hash {
            return Err(PeerError::BadInfoHash)?;
        }
        Ok(response_handshake)
    }
    async fn read_message(ref mut stream: impl Read + Write + Unpin) -> anyhow::Result<RawMessage> {
        let mut length = vec![0u8; 4];
        stream
            .read_exact(&mut length)
            .await
            .context("Failed to read message length")?;
        let length = BigEndian::read_int(&length, 4) as usize;
        let mut message_bytes = vec![0u8; length];
        stream
            .read_exact(&mut message_bytes)
            .await
            .context("Failed to read message")?;
        Ok(RawMessage::from(&message_bytes[..]))
    }
}

struct MockTcpStream {
    read_data: Vec<u8>,
    write_data: Vec<u8>,
}
impl Read for MockTcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let end = min(buf.len(), self.read_data.len());
        buf[..end].copy_from_slice(&self.read_data[..end]);
        self.get_mut().read_data = self.read_data[end..].to_vec();
        Poll::Ready(Ok(end))
    }
}
impl Write for MockTcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_mut().write_data = Vec::from(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
impl Unpin for MockTcpStream {}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_peerstream_handshake() {
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
        let response = PeerStream::handshake(&mut stream, opts).await.unwrap();
        assert_eq!(response.pstr, "test_protocol".as_bytes());
    }

    #[async_std::test]
    async fn test_peerstream_bad_info_hash() {
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
        let response = PeerStream::handshake(&mut stream, opts).await;
        assert!(response.is_err());
        assert_eq!(
            response.err().unwrap().to_string(),
            "Peer info hash mismatch"
        );
    }

    #[async_std::test]
    async fn test_peerstream_bad_protocol() {
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
        let response = PeerStream::handshake(&mut stream, opts).await;
        assert!(response.is_err());
        assert_eq!(
            response.err().unwrap().to_string(),
            "Peer protocol mismatch"
        );
    }

    #[async_std::test]
    async fn test_peerstream_read_message() {
        let mut stream = MockTcpStream {
            read_data: vec![0, 0, 0, 4, 1, 2, 2, 4],
            write_data: Vec::new(),
        };
        let response = PeerStream::read_message(&mut stream).await.unwrap();
        assert_eq!(response.message_id, 1);
        assert_eq!(response.payload, vec![2, 2, 4]);
    }

    #[async_std::test]
    async fn test_peerstream_read_message_keep_alive() {
        let mut stream = MockTcpStream {
            read_data: vec![0, 0, 0, 0],
            write_data: Vec::new(),
        };
        let response = PeerStream::read_message(&mut stream).await.unwrap();
        assert_eq!(response.message_id, 0);
        assert_eq!(response.payload, vec![]);
    }
}
