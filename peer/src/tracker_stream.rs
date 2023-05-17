use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use anyhow::Context;
use async_std::net::UdpSocket;
use byteorder::{BigEndian, ByteOrder};
use rand::Rng;

struct TrackerManager {
    connections: Vec<TrackerConnection>,
}
impl TrackerManager {
    pub async fn new() -> anyhow::Result<TrackerManager> {
        Ok(TrackerManager {
            connections: vec![],
        })
    }
    pub async fn connect(&self, addr: SocketAddr) -> anyhow::Result<TrackerConnection> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .context("Failed to establish UDP Socket")?;
        let connection_id = TrackerManager::handshake(&socket, addr).await?;
        Ok(TrackerConnection {
            addr,
            connection_id,
        })
    }
    async fn handshake(socket: &UdpSocket, addr: SocketAddr) -> anyhow::Result<i64> {
        let request = ConnectRequest::new();
        let bytes_sent = socket.send_to(&request.to_bytes(), &addr).await?;
        if bytes_sent != CONNECT_REQUEST_SIZE {
            anyhow::bail!("Unable to send connect request");
        }
        let mut bytes_recv = [0u8; CONNECT_RESPONSE_SIZE];
        loop {
            let (n, peer) = socket.recv_from(&mut bytes_recv).await?;
            if peer != addr {
                continue;
            } else if n != CONNECT_RESPONSE_SIZE {
                anyhow::bail!("Unable to read connect response");
            }
            break;
        }
        let response = ConnectResponse::from_bytes(&bytes_recv);
        if response.transaction_id != request.transaction_id {
            anyhow::bail!("Mismatched transaction ids");
        }
        Ok(response.connection_id)
    }
}

struct TrackerConnection {
    addr: SocketAddr,
    connection_id: i64,
}

#[derive(Debug)]
struct ConnectRequest {
    protocol_id: i64,
    action: u32,
    transaction_id: u32,
}

const PROTOCOL_ID: i64 = 0x41727101980;

const CONNECT_REQUEST_SIZE: usize = 16;
const CONNECT_RESPONSE_SIZE: usize = 16;
impl ConnectRequest {
    fn new() -> Self {
        Self {
            protocol_id: PROTOCOL_ID,
            action: 0,
            transaction_id: rand::random(),
        }
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 16];
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

#[derive(Debug)]
struct AnnounceResponse {
    action: u32,
    transaction_id: u32,
    interval: u32,
    leechers: u32,
    seeders: u32,
    peers: Vec<SocketAddr>,
}
impl AnnounceResponse {
    fn from_bytes(bytes: &[u8], length: usize) -> Self {
        let action = BigEndian::read_u32(&bytes[0..4]);
        let transaction_id = BigEndian::read_u32(&bytes[4..8]);
        let interval = BigEndian::read_u32(&bytes[8..12]);
        let leechers = BigEndian::read_u32(&bytes[12..16]);
        let seeders = BigEndian::read_u32(&bytes[16..20]);
        let peer_list = &bytes[20..length];
        if peer_list.len() % 6 != 0 {
            panic!("Invalid peer list size");
        }
        let mut peers = Vec::new();
        for address in peer_list.chunks(6) {
            let ip = Ipv4Addr::new(address[0], address[1], address[2], address[3]);
            let port = BigEndian::read_u16(&address[4..6]);
            let peer = SocketAddr::new(IpAddr::V4(ip), port);
            peers.push(peer);
        }
        Self {
            action,
            transaction_id,
            interval,
            leechers,
            seeders,
            peers,
        }
    }
}
