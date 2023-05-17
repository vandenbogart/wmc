use std::net::SocketAddr;


use anyhow::Context;
use async_std::net::UdpSocket;
use byteorder::{BigEndian, ByteOrder};
use rand::{Rng};

struct TrackerManager {
    socket: UdpSocket,
    connections: Vec<TrackerConnection>,
}
impl TrackerManager {
    pub async fn new() -> anyhow::Result<TrackerManager> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.context("Failed to establish UDP Socket")?;
        Ok(TrackerManager {
            socket,
            connections: vec![],
        })
    }
    pub async fn connect(&self, addr: SocketAddr) -> anyhow::Result<TrackerConnection> {
        let connection_id = TrackerManager::handshake(&self.socket, addr).await?;
        Ok(TrackerConnection {
            addr,
            connection_id,
        })
    }
    pub async fn handshake(socket: &UdpSocket, addr: SocketAddr) -> anyhow::Result<u64> {
        let transaction_id: u32 = rand::random();
        todo!() // tests for tracker


    }

}

struct TrackerConnection {
    addr: SocketAddr,
    connection_id: u64,
}

#[derive(Debug)]
struct ConnectRequest {
    protocol_id: i64,
    action: u32,
    transaction_id: u32,
}

const PROTOCOL_ID: i64 = 0x41727101980;

impl ConnectRequest {
    fn new() -> Self {
        Self {
            protocol_id: PROTOCOL_ID,
            action: 0,
            transaction_id: rand::random(),
        }
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; std::mem::size_of::<Self>()];
        BigEndian::write_i64(&mut bytes[0..8], self.protocol_id);
        BigEndian::write_u32(&mut bytes[8..12], self.action);
        BigEndian::write_u32(&mut bytes[12..16], self.transaction_id);
        bytes
    }
}

#[derive(Debug)]
struct ConnectResponse {
    action: u32,
    transaction_id: u32,
    connection_id: i64,
}
impl ConnectResponse {
    fn from_bytes(bytes: &[u8]) -> Self {
        let action = BigEndian::read_u32(&bytes[0..4]);
        let transaction_id = BigEndian::read_u32(&bytes[4..8]);
        let connection_id = BigEndian::read_i64(&bytes[8..16]);
        Self {
            action,
            transaction_id,
            connection_id,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum AnnounceEvent {
    None = 0,
    Completed,
    Started,
    Stopped,
}

#[derive(Debug)]
struct AnnounceRequest {
    connection_id: i64,
    action: u32,
    transaction_id: u32,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    downloaded: u64,
    left: u64,
    uploaded: u64,
    event: AnnounceEvent,
    ip_address: u32,
    key: u32,
    num_want: i32,
    port: u16,
}

#[derive(Debug)]
struct AnnounceRequestDescriptor {
    connection_id: i64,
    peer_id: [u8; 20],
    info_hash: [u8; 20],
    downloaded: u64,
    left: u64,
    uploaded: u64,
    event: AnnounceEvent,
}

const ANNOUNCE_REQUEST_BYTES: usize = 98;
impl AnnounceRequest {
    fn new(descriptor: AnnounceRequestDescriptor) -> Self {
        Self {
            connection_id: descriptor.connection_id,
            action: 1,
            transaction_id: rand::random(),
            info_hash: descriptor.info_hash,
            peer_id: descriptor.peer_id,
            downloaded: descriptor.downloaded,
            left: descriptor.left,
            uploaded: descriptor.uploaded,
            event: descriptor.event,
            ip_address: 0,
            key: rand::random(),
            num_want: -1,
            port: 6881,
        }
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; ANNOUNCE_REQUEST_BYTES];
        BigEndian::write_i64(&mut bytes[0..8], self.connection_id);
        BigEndian::write_u32(&mut bytes[8..12], self.action);
        BigEndian::write_u32(&mut bytes[12..16], self.transaction_id);
        bytes[16..36].copy_from_slice(&self.info_hash);
        bytes[36..56].copy_from_slice(&self.peer_id);
        BigEndian::write_u64(&mut bytes[56..64], self.downloaded);
        BigEndian::write_u64(&mut bytes[64..72], self.left);
        BigEndian::write_u64(&mut bytes[72..80], self.uploaded);
        BigEndian::write_u32(&mut bytes[80..84], self.event as u32);
        BigEndian::write_u32(&mut bytes[84..88], self.ip_address);
        BigEndian::write_u32(&mut bytes[88..92], self.key);
        BigEndian::write_i32(&mut bytes[92..96], self.num_want);
        BigEndian::write_u16(&mut bytes[96..98], self.port);
        bytes
    }
}
