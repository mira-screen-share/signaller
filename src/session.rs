use std::collections::HashSet;
use std::time::SystemTime;

pub struct Session {
    pub sharer: String,
    pub viewers: HashSet<String>,
    pub start_time: SystemTime,
}

impl Session {
    pub fn new(sharer: String) -> Self {
        Session {
            sharer,
            viewers: Default::default(),
            start_time: SystemTime::now(),
        }
    }
}
