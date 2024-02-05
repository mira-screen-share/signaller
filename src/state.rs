use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use base64::Engine;
use failure::{format_err, Error};
use futures_channel::mpsc::UnboundedSender;
use log::info;
use tokio::sync::Mutex;
use tungstenite::protocol::Message;
use twilio::TwilioAuthentication;

use crate::config::Config;
use crate::metrics;
use crate::peer::{Peer, PeerType};
use crate::session::Session;
use crate::signaller_message::{IceServer, SignallerMessage};
use crate::twilio_helper::get_twilio_ice_servers;

type Result<T> = std::result::Result<T, Error>;
type Tx = UnboundedSender<Message>;

pub struct State {
    pub sessions: HashMap<String, Session>,
    pub sharer_ip_to_room: HashMap<SocketAddr, String>,
    pub peers: HashMap<String, Peer>,
    pub twilio_client: Option<twilio::TwilioClient>,
    pub twilio_account_sid: Option<String>,
}

pub type StateType = Arc<Mutex<State>>;

impl State {
    pub fn new(config: &Config) -> StateType {
        let base64_engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::general_purpose::PAD,
        );
        Arc::new(Mutex::new(State {
            sessions: Default::default(),
            sharer_ip_to_room: Default::default(),
            peers: Default::default(),
            twilio_client: {
                if let (Some(account_sid), Some(auth_token)) =
                    (&config.twilio_account_sid, &config.twilio_auth_token)
                {
                    Some(twilio::TwilioClient::new(
                        "https://api.twilio.com",
                        TwilioAuthentication::BasicAuth {
                            basic_auth: base64_engine
                                .encode(format!("{}:{}", account_sid, auth_token).as_bytes()),
                        },
                    ))
                } else {
                    None
                }
            },
            twilio_account_sid: config.twilio_account_sid.clone(),
        }))
    }

    pub fn add_sharer(&mut self, room: String, sender: Tx, ip: &SocketAddr) -> Result<()> {
        if self.sessions.contains_key(&room) {
            return Err(format_err!("room already exists"));
        }
        self.sessions
            .insert(room.clone(), Session::new(room.clone(), ip.clone()));
        self.sharer_ip_to_room.insert(ip.clone(), room.clone());
        metrics::NUM_ONGOING_SESSIONS.inc();
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

    fn remove_session(&mut self, room: &String) {
        info!("Removing session {}", room);
        let session = self.sessions.remove(room).unwrap();
        self.sharer_ip_to_room.remove(&session.sharer_ip);
        let duration_sec = session.start_time.elapsed().unwrap().as_secs_f64();
        metrics::NUM_ONGOING_SESSIONS.dec();
        metrics::SESSION_DURATION_SEC.observe(duration_sec);
        for viewer in session.viewers {
            self.peers[&viewer]
                .sender
                .unbounded_send(Message::Text(
                    serde_json::to_string(&SignallerMessage::RoomClosed {
                        to: viewer.clone(),
                        room: room.clone(),
                    })
                    .unwrap(),
                ))
                .unwrap();
            self.peers.remove(&viewer);
        }
        self.peers.remove(&session.sharer);
    }

    /// Leave a session. id is the id of the viewer or the sharer.
    pub fn leave_session(&mut self, id: String) -> Result<()> {
        if self.sessions.contains_key(&id) {
            // id is host. remove session
            self.remove_session(&id);
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

    pub fn on_disconnect(&mut self, ip_hash: &SocketAddr) {
        if let Some(room) = self.sharer_ip_to_room.get(ip_hash) {
            self.remove_session(&room.clone());
        }
    }

    pub fn get_room_id_from_peer_uuid(&self, viewer_uuid: &String) -> Result<String> {
        let peer = self
            .peers
            .get(viewer_uuid)
            .ok_or_else(|| format_err!("Peer does not exist"))?;
        Ok(peer.room.clone())
    }

    pub async fn get_ice_servers(&self) -> Vec<IceServer> {
        if let (Some(client), Some(sid)) = (&self.twilio_client, &self.twilio_account_sid) {
            get_twilio_ice_servers(client, sid).await
        } else {
            vec![]
        }
    }
}
