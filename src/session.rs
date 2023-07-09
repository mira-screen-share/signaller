use std::collections::HashSet;

pub struct Session {
    pub sharer: String,
    pub viewers: HashSet<String>,
}

impl Session {
    pub fn new(sharer: String) -> Self {
        Session {
            sharer,
            viewers: Default::default(),
        }
    }
}
