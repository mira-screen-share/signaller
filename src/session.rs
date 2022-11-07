use std::collections::HashSet;
use uuid::Uuid;

pub struct Session {
    pub sharer: Uuid,
    pub viewers: HashSet<Uuid>,
}

impl Session {
    pub fn new(sharer: Uuid) -> Self {
        Session {
            sharer,
            viewers: Default::default(),
        }
    }
}
