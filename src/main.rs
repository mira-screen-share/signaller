use std::net::SocketAddr;
use std::path::Path;

use clap::Parser;
use failure::{Error, format_err};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use log::info;
use rand::{Rng, thread_rng};
use rand::distributions::Distribution;
use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;

use crate::signaller_message::SignallerMessage;
use crate::state::StateType;

mod peer;
mod session;
mod signaller_message;
mod state;
mod twilio_helper;
mod config;
mod args;

type Result<T> = std::result::Result<T, Error>;
type Tx = UnboundedSender<Message>;

const ROOM_ID_LEN: usize = 5;

fn generate_room_id(len: usize) -> String {
    pub struct UserFriendlyAlphabet;
    impl Distribution<u8> for UserFriendlyAlphabet {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
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

async fn handle_message(state: &mut state::State, tx: &Tx, raw_payload: &str) -> Result<()> {
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
            )?)).unwrap_or_else(|e| {
                info!("Error sending start response: {}", e);
            });
        }
        SignallerMessage::Leave { from } => {
            forward_message(state, state.get_room_id_from_peer_uuid(&from)?)?;
            state.leave_session(from)?;
        }
        SignallerMessage::IceServers {} => {
            let ice_servers = state.get_ice_servers().await;
            tx.unbounded_send(Message::Text(serde_json::to_string(
                &SignallerMessage::IceServersResponse { ice_servers },
            )?)).unwrap_or_else(|e| {
                info!("Error sending ice server response: {}", e);
            });
        }
        SignallerMessage::Offer { from: _, to }
        | SignallerMessage::Answer { from: _, to }
        | SignallerMessage::Ice { from: _, to }
        | SignallerMessage::JoinDeclined { to } => {
            forward_message(state, to)?;
        }
        SignallerMessage::KeepAlive {}
        | SignallerMessage::StartResponse { .. }
        | SignallerMessage::IceServersResponse { .. } => {}
    };
    Ok(())
}

async fn process_message(msg: Message, state: StateType, tx: &Tx) -> std::result::Result<(), tungstenite::Error> {
    if !msg.is_text() {
        return Ok(());
    }

    if let Ok(s) = msg.to_text() {
        let mut locked_state = state.lock().await;
        if let Err(e) = handle_message(&mut locked_state, &tx, s).await {
            info!(
                    "Error occurred when handling message: {}\nMessage: {}",
                    e,
                    msg.to_string()
                );
        }
    }
    Ok(())
}

async fn handle_connection(state: StateType, raw_stream: TcpStream, addr: SocketAddr) {
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
        process_message(msg, state.clone(), &tx)
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
    let args = args::Args::parse();
    let address = args.address;
    let config_path = args.config;
    let config = config::load(Path::new(&config_path))?;

    let state = state::State::new(&config);

    // Create the event loop and TCP listener we'll accept connections on.
    let listener = TcpListener::bind(&address).await.expect("Failed to bind");
    info!("Listening on: {}", address);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}
