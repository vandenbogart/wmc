use std::io::{Read, Write};
use std::net::{UdpSocket, ToSocketAddrs, Ipv4Addr, TcpStream, SocketAddr};
use std::path::{self, Path};
use std::fs::{self, File};
use std::str::{FromStr, from_utf8};
use std::{u64, i64, u16};

use byteorder::{BigEndian, ByteOrder};
use rand::Rng;
use url::Url;
use urlencoding::decode;

#[derive(Debug)]
struct MagnetLink {
    exact_topic: [u8; 20],
    display_name: String,
    trackers: Vec<Tracker>,
}

#[derive(Debug)]
enum TrackerProtocol {
    UDP,
    HTTP,
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
    Stopped
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
struct PeerAddress {
    address: Ipv4Addr,
    port: u16
}
impl PeerAddress {
    fn from_bytes(bytes: &[u8]) -> Self {
        let address = Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]);
        Self {
            address,
            port: BigEndian::read_u16(&bytes[4..6]),
        }
    }
    fn to_host_port(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

#[derive(Debug)]
struct AnnounceResponse {
    action: u32,
    transaction_id: u32,
    interval: u32,
    leechers: u32,
    seeders: u32,
    peers: Vec<PeerAddress>,
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
            let peer = PeerAddress::from_bytes(address);
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

#[derive(Debug)]
struct Tracker {
    protocol: TrackerProtocol,
    host: String,
    port: u16,
}
impl Tracker {
    fn from_magnet_link(tr: &str) -> Self {
        let url = Url::from_str(tr).ok().unwrap();
        Tracker {
            protocol: match url.scheme() {
                "udp" => TrackerProtocol::UDP,
                "http" => TrackerProtocol::HTTP,
                &_ => panic!("Unhandled tracker protocol: {}", url.scheme())
            },
            host: url.host_str().unwrap().into(),
            port: url.port().unwrap_or(80),
        }
    }
    fn to_host_port(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

const BITTORRENT_PROTOCOL: &str = "BitTorrent protocol";
const PSTR_LEN_BYTE: u8 = 19;
const INFO_HASH_LEN: usize = 20;
const PEER_ID_LEN: usize = 20;
const PEER_CONNECTION_REQUEST_LEN: usize = 68;

#[derive(Debug)]
struct PeerConnectionData {
    pstr_len: usize,
    pstr: [u8; 19],
    reserved: [u8; 8],
    info_hash: [u8; INFO_HASH_LEN],
    peer_id: [u8; PEER_ID_LEN],
}
impl PeerConnectionData {
    fn new(info_hash: [u8; INFO_HASH_LEN], peer_id: [u8; PEER_ID_LEN]) -> Self {
        let mut pstr = [0u8; PSTR_LEN_BYTE as usize];
        pstr.copy_from_slice(BITTORRENT_PROTOCOL.as_bytes());
        Self {
            pstr_len: 19,
            pstr,
            reserved: [0u8; 8],
            info_hash,
            peer_id,
        }
    }
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; PEER_CONNECTION_REQUEST_LEN];
        BigEndian::write_int(&mut bytes, self.pstr_len as i64, 1);
        bytes[1..20].copy_from_slice(&self.pstr);
        bytes[20..28].copy_from_slice(&self.reserved);
        bytes[28..48].copy_from_slice(&self.info_hash);
        bytes[48..68].copy_from_slice(&self.peer_id);
        bytes
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let pstr_len = BigEndian::read_int(&bytes, 1) as usize;
        let mut pstr = [0u8; 19];
        let mut reserved = [0u8; 8];
        let mut info_hash = [0u8; INFO_HASH_LEN];
        let mut peer_id = [0u8; PEER_ID_LEN];
        pstr.copy_from_slice(&bytes[1..20]);
        reserved.copy_from_slice(&bytes[20..28]);
        info_hash.copy_from_slice(&bytes[28..48]);
        peer_id.copy_from_slice(&bytes[48..68]);
        Self {
            pstr_len,
            pstr,
            reserved,
            info_hash,
            peer_id,
        }
    }

}

fn main() -> anyhow::Result<()> {
    let magnet = "magnet:?xt=urn:btih:62B9305B850F2219B960929EC4CBD2E826004D73&dn=Eminem+-+Curtain+Call+2+%28Explicit%29+%282022%29+Mp3+320kbps+%5BPMEDIA%5D+%E2%AD%90%EF%B8%8F&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A6969%2Fannounce&tr=udp%3A%2F%2Fopen.demonii.com%3A1337&tr=udp%3A%2F%2F9.rarbg.me%3A2980%2Fannounce&tr=udp%3A%2F%2Fexodus.desync.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.moeking.me%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce&tr=udp%3A%2F%2Fexplodie.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fretracker.lanta-net.ru%3A2710%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=http%3A%2F%2Ftracker.files.fm%3A6969%2Fannounce&tr=udp%3A%2F%2Ffe.dealclub.de%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.leech.ie%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=http%3A%2F%2Ftracker.openbittorrent.com%3A80%2Fannounce&tr=udp%3A%2F%2Fopentracker.i2p.rocks%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.internetwarriors.net%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.leechers-paradise.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fcoppersurfer.tk%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.zer0day.to%3A1337%2Fannounce";

    let decoded = decode(&magnet)?.into_owned();
    let slice = &decoded[8..];
    let split = slice.split("&").collect::<Vec<_>>();

    let mut trackers = Vec::new();
    let mut exact_topic = [0u8; 20];
    let mut display_name = String::new();
    for item in split {
        let (id, value) = item.split_once("=").unwrap();
        match id {
            "xt" => {
                let info_string = value[value.len() - 40..].as_bytes();
                let bytes = hex::decode(info_string)?;
                exact_topic.copy_from_slice(bytes.as_slice());
            },
            "dn" => {
                display_name = String::from(value);
            },
            "tr" => {
                let tracker = Tracker::from_magnet_link(value);
                trackers.push(tracker);
            },
            &_ => (),
        }
    }
    let link = MagnetLink {
        exact_topic,
        display_name,
        trackers,
    };

    let tracker = &link.trackers[0];
    let client_socket = UdpSocket::bind("0.0.0.0:0")?;

    let request = ConnectRequest::new();
    dbg!(&request);
    client_socket.send_to(request.to_bytes().as_slice(), tracker.to_host_port())?;

    let mut buffer = [0u8; 4096];
    let (number_of_bytes, src_addr) = client_socket.recv_from(&mut buffer)?;

    if number_of_bytes != 16 {
        panic!("Invalid response from tracker");
    }

    let response = ConnectResponse::from_bytes(&buffer);
    dbg!(&response);

    let connection_id = response.connection_id;

    let mut peer_id = [0u8; 20];
    rand::thread_rng().fill(&mut peer_id[..]);
    let signature = "-WM0001-";
    peer_id[0..signature.len()].copy_from_slice(signature.as_bytes());
    let info_hash = link.exact_topic;


    let announce_request = AnnounceRequest::new(AnnounceRequestDescriptor {
        connection_id,
        peer_id,
        info_hash,
        downloaded: 0,
        left: 0,
        uploaded: 0,
        event: AnnounceEvent::None,
    });

    dbg!(&announce_request);

    client_socket.send_to(announce_request.to_bytes().as_slice(), tracker.to_host_port())?;

    let mut buffer = [0u8; 4096];
    let (number_of_bytes, src_addr) = client_socket.recv_from(&mut buffer)?;

    let announce_response = AnnounceResponse::from_bytes(&buffer, number_of_bytes);
    dbg!(&announce_response);

    let peer = &announce_response.peers[2];

    dbg!(peer);
    let mut stream = TcpStream::connect("70.81.126.161:2372")?;
    println!("Successfully connected to server at {}", peer.address);

    let request = PeerConnectionData::new(info_hash, peer_id);
    dbg!(&request);
    stream.write_all(&request.to_bytes()).expect("Failed to send data");

    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(number_of_bytes) => {
            let response = PeerConnectionData::from_bytes(&buffer[..number_of_bytes]);
            if request.info_hash != response.info_hash {
                panic!("Mismatched info hash");
            }
            dbg!(&response);
            println!(
                "Received {} bytes: {}",
                number_of_bytes,
                String::from_utf8_lossy(&buffer[..number_of_bytes])
            );
        }
        Err(e) => {
            eprintln!("Failed to receive data: {}", e);
        }
    }


    Ok(())
}
