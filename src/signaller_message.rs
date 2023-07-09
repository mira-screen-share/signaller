use serde::{Deserialize, Serialize};

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
    },
    Start {},
    StartResponse {
        room: String,
    },
    Leave {
        from: String,
    },
    KeepAlive {},
}
