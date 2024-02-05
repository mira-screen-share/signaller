use futures_channel::mpsc::UnboundedSender;
use serde::{Deserialize, Serialize};
use warp::ws::Message;

type Tx = UnboundedSender<Message>;

pub struct Peer {
    pub room: String,
    pub sender: Tx,
    pub peer_type: PeerType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PeerType {
    Sharer {},
    Viewer {},
}
