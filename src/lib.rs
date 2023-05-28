use async_std::{task, future::ready};
use futures::{join, stream::FuturesUnordered, StreamExt};
use peer::{magnet::Magnet, tracker_stream::{TrackerManager, TrackerConnection}};

mod peer;

pub struct TRipClient {
    magnet: Magnet,
    trackers: Vec<TrackerConnection>,
}
impl TRipClient {
    pub fn new(link: &str) -> anyhow::Result<Self> {
        let magnet = Magnet::from_link(link)?;
        let futures = FuturesUnordered::new();
        for tracker in magnet.trackers.iter() {
            {
                let future = TrackerManager::connect(tracker.clone());
                futures.push(future);
            }
        }
        let mut connections = vec![];
        task::block_on(async {
            futures.for_each(|output| {
                match output {
                    Ok(conn) => {
                        println!("Connected to {}", conn.addr);
                        connections.push(conn);
                    }
                    Err(e) => {
                        println!("{}", e);
                    },
                }
                ready(())
            }).await
        });
        Ok(Self {
            magnet,
            trackers: connections,
        })
    }
}
