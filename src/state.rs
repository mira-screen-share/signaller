use crate::peer::{Peer, PeerType};
use crate::session::Session;
use failure::{format_err, Error};
use futures_channel::mpsc::UnboundedSender;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tungstenite::protocol::Message;

type Result<T> = std::result::Result<T, Error>;
type Tx = UnboundedSender<Message>;

pub struct State {
    pub sessions: HashMap<String, Session>,
    pub peers: HashMap<String, Peer>,
}

pub type StateType = Arc<Mutex<State>>;

impl State {
    pub fn new() -> StateType {
        Arc::new(Mutex::new(State {
            sessions: Default::default(),
            peers: Default::default(),
        }))
    }

    pub fn add_sharer(&mut self, room: String, sender: Tx) -> Result<()> {
        if self.sessions.contains_key(&room) {
            return Err(format_err!("room already exists"));
        }
        self.sessions
            .insert(room.clone(), Session::new(room.clone()));
        self.peers.insert(
            room.clone(),
            Peer {
                room,
                sender,
                peer_type: PeerType::Sharer {},
            },
        );
        Ok(())
    }

    pub fn add_viewer(&mut self, id: String, room: String, sender: Tx) -> Result<()> {
        if !self.sessions.contains_key(&room) {
            return Err(format_err!("room does not exist"));
        }
        self.sessions
            .get_mut(&room)
            .unwrap()
            .viewers
            .insert(id.clone());
        self.peers.insert(
            id,
            Peer {
                room,
                sender,
                peer_type: PeerType::Viewer {},
            },
        );
        Ok(())
    }

    /// Leave a session. id is the id of the viewer or the sharer.
    pub fn leave_session(&mut self, id: String) -> Result<()> {
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
            let session = self.sessions.get_mut(&peer.room).unwrap();
            session.viewers.remove(&id);
            self.peers.remove(&id);
        }
        Ok(())
    }
}
