use std::{fmt::Display, str::FromStr};
use url::Url;

#[derive(Debug, PartialEq)]
struct InfoHash {
    pub bytes: [u8; 20],
}
impl Display for InfoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = String::from_utf8_lossy(&self.bytes);
        f.write_str(&str)
    }
}

struct Magnet {
    pub info_hash: InfoHash,
    pub display_name: String,
    pub trackers: Vec<Url>,
}

impl Magnet {
    fn from_link(link: &str) -> anyhow::Result<Self> {
        let decoded = urlencoding::decode(link)?;
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
                }
                "dn" => {
                    display_name = String::from(value);
                }
                "tr" => {
                    if let Some(tracker) = Url::from_str(value).ok() {
                        trackers.push(tracker);
                    }
                }
                &_ => (),
            }
        }
        Ok(Self {
            info_hash: InfoHash { bytes: exact_topic },
            display_name,
            trackers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_info_hash() {
        let link = "magnet:?xt=urn:btih:62B9305B850F2219B960929EC4CBD2E826004D73&dn=Eminem+-+Curtain+Call+2+%28Explicit%29+%282022%29+Mp3+320kbps+%5BPMEDIA%5D+%E2%AD%90%EF%B8%8F&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A6969%2Fannounce&tr=udp%3A%2F%2Fopen.demonii.com%3A1337&tr=udp%3A%2F%2F9.rarbg.me%3A2980%2Fannounce&tr=udp%3A%2F%2Fexodus.desync.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.moeking.me%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce&tr=udp%3A%2F%2Fexplodie.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fretracker.lanta-net.ru%3A2710%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=http%3A%2F%2Ftracker.files.fm%3A6969%2Fannounce&tr=udp%3A%2F%2Ffe.dealclub.de%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.leech.ie%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=http%3A%2F%2Ftracker.openbittorrent.com%3A80%2Fannounce&tr=udp%3A%2F%2Fopentracker.i2p.rocks%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.internetwarriors.net%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.leechers-paradise.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fcoppersurfer.tk%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.zer0day.to%3A1337%2Fannounce";
        let magnet = Magnet::from_link(&link).unwrap();
        let encoded = hex::encode(magnet.info_hash.bytes).to_uppercase();
        assert_eq!(encoded, "62B9305B850F2219B960929EC4CBD2E826004D73");
    }

    #[test]
    fn test_parse_display_name() {
        let link = "magnet:?xt=urn:btih:62B9305B850F2219B960929EC4CBD2E826004D73&dn=Eminem+-+Curtain+Call+2+%28Explicit%29+%282022%29+Mp3+320kbps+%5BPMEDIA%5D+%E2%AD%90%EF%B8%8F&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A6969%2Fannounce&tr=udp%3A%2F%2Fopen.demonii.com%3A1337&tr=udp%3A%2F%2F9.rarbg.me%3A2980%2Fannounce&tr=udp%3A%2F%2Fexodus.desync.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.moeking.me%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce&tr=udp%3A%2F%2Fexplodie.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fretracker.lanta-net.ru%3A2710%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=http%3A%2F%2Ftracker.files.fm%3A6969%2Fannounce&tr=udp%3A%2F%2Ffe.dealclub.de%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.leech.ie%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=http%3A%2F%2Ftracker.openbittorrent.com%3A80%2Fannounce&tr=udp%3A%2F%2Fopentracker.i2p.rocks%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.internetwarriors.net%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.leechers-paradise.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fcoppersurfer.tk%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.zer0day.to%3A1337%2Fannounce";
        let magnet = Magnet::from_link(&link).unwrap();
        let expected = "Eminem+-+Curtain+Call+2+(Explicit)+(2022)+Mp3+320kbps+[PMEDIA]+⭐\u{fe0f}";
        assert_eq!(magnet.display_name, expected);
    }

    #[test]
    fn test_parse_trackers() {
        let link = "magnet:?xt=urn:btih:62B9305B850F2219B960929EC4CBD2E826004D73&dn=Eminem+-+Curtain+Call+2+%28Explicit%29+%282022%29+Mp3+320kbps+%5BPMEDIA%5D+%E2%AD%90%EF%B8%8F&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A6969%2Fannounce&tr=udp%3A%2F%2Fopen.demonii.com%3A1337&tr=udp%3A%2F%2F9.rarbg.me%3A2980%2Fannounce&tr=udp%3A%2F%2Fexodus.desync.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.moeking.me%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce&tr=udp%3A%2F%2Fexplodie.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fretracker.lanta-net.ru%3A2710%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=http%3A%2F%2Ftracker.files.fm%3A6969%2Fannounce&tr=udp%3A%2F%2Ffe.dealclub.de%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.leech.ie%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337%2Fannounce&tr=http%3A%2F%2Ftracker.openbittorrent.com%3A80%2Fannounce&tr=udp%3A%2F%2Fopentracker.i2p.rocks%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.internetwarriors.net%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.leechers-paradise.org%3A6969%2Fannounce&tr=udp%3A%2F%2Fcoppersurfer.tk%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.zer0day.to%3A1337%2Fannounce";
        let magnet = Magnet::from_link(&link).unwrap();
        assert!(magnet.trackers.len() == 21);
        assert_eq!(
            magnet.trackers.first().unwrap().as_str(),
            "udp://tracker.opentrackr.org:1337/announce"
        );
    }
}
