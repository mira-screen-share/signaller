use crate::peer::{Peer, PeerType};
use crate::session::Session;
use failure::{format_err, Error};
use futures_channel::mpsc::UnboundedSender;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tungstenite::protocol::Message;
use uuid::Uuid;

type Result<T> = std::result::Result<T, Error>;
type Tx = UnboundedSender<Message>;

pub struct State {
    pub sessions: HashMap<Uuid, Session>,
    pub peers: HashMap<Uuid, Peer>,
}

pub type StateType = Arc<Mutex<State>>;

impl State {
    pub fn new() -> StateType {
        Arc::new(Mutex::new(State {
            sessions: Default::default(),
            peers: Default::default(),
        }))
    }

    pub fn add_sharer(&mut self, id: Uuid, sender: Tx) -> Result<()> {
        if self.sessions.contains_key(&id) {
            return Err(format_err!("Session already exists"));
        }
        self.sessions.insert(id, Session::new(id));
        self.peers.insert(
            id,
            Peer {
                session: id,
                sender,
                peer_type: PeerType::Sharer {},
            },
        );
        Ok(())
    }

    pub fn add_viewer(&mut self, id: Uuid, session: Uuid, sender: Tx) -> Result<()> {
        if !self.sessions.contains_key(&session) {
            return Err(format_err!("Session does not exist"));
        }
        self.sessions.get_mut(&session).unwrap().viewers.insert(id);
        self.peers.insert(
            id,
            Peer {
                session,
                sender,
                peer_type: PeerType::Viewer {},
            },
        );
        Ok(())
    }

    pub fn end_session(&mut self, id: Uuid) -> Result<()> {
        // todo: remove all peers from session (if exists), then remove session
        let session_id = self.peers.get(&id).unwrap().session;
        if !self.sessions.contains_key(&session_id) {
            return Err(format_err!("Session does not exist"));
        }
        let session = self.sessions.remove(&session_id).unwrap();
        for viewer in session.viewers {
            self.peers.remove(&viewer);
        }
        self.peers.remove(&session.sharer);
        Ok(())
    }
}
