use uuid::Uuid;
use futures_channel::mpsc::{unbounded, UnboundedSender};
use tungstenite::protocol::Message;
use serde::{Serialize, Deserialize};

type Tx = UnboundedSender<Message>;



pub struct Peer {
    pub session: Uuid,
    pub sender: Tx,
    pub peer_type: PeerType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PeerType {
    Sharer {},
    Viewer {}
}