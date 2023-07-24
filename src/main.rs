mod peer;
mod session;
mod signaller_message;
mod state;

use std::{env, net::SocketAddr};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use crate::signaller_message::SignallerMessage;
use failure::{format_err, Error};
use log::info;

use rand::distributions::{Alphanumeric, Distribution};
use rand::{thread_rng, Rng};
use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;

type Result<T> = std::result::Result<T, Error>;
type Tx = UnboundedSender<Message>;

const ROOM_ID_LEN: usize = 5;

fn generate_room_id(len: usize) -> String {
    pub struct UserFriendlyAlphabet;
    impl Distribution<u8> for UserFriendlyAlphabet {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
            const RANGE: u32 = 32;
            const GEN_ASCII_STR_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
            GEN_ASCII_STR_CHARSET[(rng.next_u32() >> (32 - 5)) as usize]
        }
    }

    thread_rng()
        .sample_iter(&UserFriendlyAlphabet)
        .take(len)
        .map(char::from)
        .collect()
}

fn handle_message(state: &mut state::State, tx: &Tx, raw_payload: &str) -> Result<()> {
    let msg: SignallerMessage = serde_json::from_str(raw_payload)?;
    let forward_message = |state: &state::State, to: String| -> Result<()> {
        let peer = state
            .peers
            .get(&to)
            .ok_or_else(|| format_err!("Peer does not exist"))?;
        peer.sender.unbounded_send(raw_payload.into())?;
        Ok(())
    };

    match msg {
        SignallerMessage::Join { from, room } => {
            state.add_viewer(from, room.clone(), tx.clone())?;
            forward_message(state, room)?;
        }
        SignallerMessage::Start {} => {
            let tries = 3;
            let mut room = generate_room_id(ROOM_ID_LEN);
            for _ in 0..tries {
                if !state.sessions.contains_key(&room) {
                    break;
                }
                room = generate_room_id(ROOM_ID_LEN);
            }
            info!("New room: {}", room);
            state.add_sharer(room.clone(), tx.clone())?;
            tx.unbounded_send(Message::Text(serde_json::to_string(
                &SignallerMessage::StartResponse { room },
            )?))
            .unwrap_or_else(|e| {
                info!("Error sending start response: {}", e);
            });
        }
        SignallerMessage::Leave { from } => {
            state.leave_session(from)?;
        }
        SignallerMessage::Offer { from: _, to }
        | SignallerMessage::Answer { from: _, to }
        | SignallerMessage::Ice { from: _, to }
        | SignallerMessage::JoinDeclined { to } => {
            forward_message(state, to)?;
        }
        SignallerMessage::KeepAlive {} | SignallerMessage::StartResponse { .. } => {}
    };
    Ok(())
}

async fn handle_connection(state: state::StateType, raw_stream: TcpStream, addr: SocketAddr) {
    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await {
        Ok(ws_stream) => ws_stream,
        Err(_e) => {
            return; // silently return if the incoming connection does not use ws protocol
        }
    };

    info!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();

    let (outgoing, incoming) = ws_stream.split();

    let handle_incoming = incoming.try_for_each(|msg| {
        if !msg.is_text() {
            return future::ok(());
        }

        if let Ok(s) = msg.to_text() {
            let mut locked_state = state.lock().unwrap();
            if let Err(e) = handle_message(&mut locked_state, &tx, s) {
                info!(
                    "Error occurred when handling message: {}\nMessage: {}",
                    e,
                    msg.to_string()
                );
            }
        }
        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(handle_incoming, receive_from_others);
    future::select(handle_incoming, receive_from_others).await;

    info!("{} disconnected", &addr);
    //locked_state.remove(&addr); todo: handle termination logic
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    let state = state::State::new();

    // Create the event loop and TCP listener we'll accept connections on.
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    info!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}
