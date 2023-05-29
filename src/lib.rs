use std::{net::SocketAddr, collections::HashSet};

use async_std::{future::ready, task};
use futures::{join, stream::FuturesUnordered, StreamExt};
use peer::{
    magnet::Magnet,
    peer_stream::PeerConnection,
    tracker_stream::{AnnounceEvent, AnnounceRequestDescriptor, TrackerConnection},
};
use rand::Rng;
use url::Url;

mod peer;

struct Peers {
    connections: Vec<PeerConnection>,
}

struct Trackers {
    pub connections: Vec<TrackerConnection>,
}
impl Trackers {
    fn new(tracker_addrs: &Vec<Url>) -> Self {
        let futures = tracker_addrs
            .iter()
            .map(|tracker| TrackerConnection::new(tracker.clone()))
            .collect::<FuturesUnordered<_>>();
        let resolved = task::block_on(async { futures.collect::<Vec<_>>().await });
        let conns = resolved
            .into_iter()
            .filter_map(|conn| match conn {
                Ok(conn) => {
                    println!("Connected to {}", conn.addr);
                    Some(conn)
                }
                Err(e) => {
                    println!("Tracker connection timed out");
                    None
                }
            })
            .collect();
        Self { connections: conns }
    }
    async fn announce(&self, peer_id: [u8; 20], info_hash: [u8; 20]) -> Vec<SocketAddr> {
        let futures = FuturesUnordered::new();
        for conn in self.connections.iter() {
            futures.push(conn.announce(AnnounceRequestDescriptor {
                connection_id: conn.connection_id,
                peer_id,
                info_hash,
                downloaded: 0,
                left: 0,
                uploaded: 0,
                event: AnnounceEvent::None,
            }))
        }
        let resolved = futures.filter_map(|result| {
            async {
                match result {
                    Ok(resp) => Some(resp),
                    Err(_) => {
                        println!("Failed to announce to tracker");
                        None
                    },
                }
            }
        }).collect::<Vec<_>>().await;
        let mut uniques = HashSet::new();
        let mut flattened = resolved.into_iter().flatten().collect::<Vec<_>>();
        flattened.retain(|i| uniques.insert(*i));
        flattened
    }
}

pub struct TRipClient {
    magnet: Magnet,
}
impl TRipClient {
    pub fn new(link: &str) -> anyhow::Result<Self> {
        let magnet = Magnet::from_link(link)?;
        let trackers = Trackers::new(&magnet.trackers);
        let mut peer_id = [0u8; 20];
        rand::thread_rng().fill(&mut peer_id[..]);
        let signature = "-WM0001-";
        peer_id[0..signature.len()].copy_from_slice(signature.as_bytes());

        let result = task::block_on(trackers.announce(peer_id, magnet.info_hash.bytes));
        dbg!(result);
        Ok(Self { magnet })
    }
}
