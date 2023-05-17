/*
 * Peer Communication
 *
 * This crate abstracts communication with peers similarly to how TcpStream
 * wraps network communication.
 */
mod messages;
mod peer_stream;
mod tracker_stream;

#[cfg(test)]
mod tests {

    #[test]
    fn test() {
    }
}

