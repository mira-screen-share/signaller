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

    /// Leave a session. id is the uuid of the viewer or the sharer.
    pub fn leave_session(&mut self, id: Uuid) -> Result<()> {
        if self.sessions.contains_key(&id) {
            // id is host. remove session
            let session = self.sessions.remove(&id).unwrap();
            for viewer in session.viewers {
                self.peers[&viewer]
                    .sender
                    .unbounded_send(Message::Close(None))?;
                self.peers.remove(&viewer);
            }
            self.peers.remove(&session.sharer);
        } else {
            let peer = self
                .peers
                .get(&id)
                .ok_or_else(|| format_err!("Peer does not exist"))?;
            let session = self.sessions.get_mut(&peer.session).unwrap();
            session.viewers.remove(&id);
            self.peers.remove(&id);
        }
        Ok(())
    }
}
