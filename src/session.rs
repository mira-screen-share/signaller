use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::SystemTime;

pub struct Session {
    pub sharer: String,
    pub viewers: HashSet<String>,
    pub start_time: SystemTime,
    pub sharer_socket_addr: SocketAddr,
}

impl Session {
    pub fn new(sharer: String, sharer_socket_addr: SocketAddr) -> Self {
        Session {
            sharer,
            viewers: Default::default(),
            start_time: SystemTime::now(),
            sharer_socket_addr,
        }
    }
}
