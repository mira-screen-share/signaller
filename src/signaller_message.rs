use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct IceServer {
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignallerMessage {
    Offer {
        // sdp: RTCSessionDescription,
        from: String,
        to: String,
    },
    Answer {
        // sdp: RTCSessionDescription,
        from: String,
        to: String,
    },
    Ice {
        // ice: RTCIceCandidateInit,
        from: String,
        to: String,
    },
    Join {
        from: String,
        room: String,
    },
    JoinDeclined {
        to: String,
        reason: String,
    },
    Start {},
    StartResponse {
        room: String,
    },
    Leave {
        from: String,
    },
    RoomClosed {
        to: String,
        room: String,
    },
    KeepAlive {},
    IceServers {},
    IceServersResponse {
        ice_servers: Vec<IceServer>,
    },
}
