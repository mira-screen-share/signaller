use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignallerMessage {
    Offer {
        // sdp: RTCSessionDescription,
        uuid: String,
        to: String,
    },
    Answer {
        // sdp: RTCSessionDescription,
        uuid: String,
        to: String,
    },
    Ice {
        // ice: RTCIceCandidateInit,
        uuid: String,
        to: String,
    },
    Join {
        uuid: String,
        room: String,
    },
    Start {
        uuid: String,
    },
    Leave {
        uuid: String,
    },
    KeepAlive {},
}
