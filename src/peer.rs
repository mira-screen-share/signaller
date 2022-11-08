use futures_channel::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};
use tungstenite::protocol::Message;
use uuid::Uuid;

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
    Viewer {},
}
