use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::SystemTime;

pub struct Session {
    pub sharer: String,
    pub viewers: HashSet<String>,
    pub start_time: SystemTime,
    pub sharer_ip: SocketAddr,
}

impl Session {
    pub fn new(sharer: String, sharer_ip: SocketAddr) -> Self {
        Session {
            sharer,
            viewers: Default::default(),
            start_time: SystemTime::now(),
            sharer_ip,
        }
    }
}
