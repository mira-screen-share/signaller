use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::str::FromStr;

use clap::Parser;
use failure::{format_err, Error};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use log::info;
use rand::distributions::Distribution;
use rand::{thread_rng, Rng};
use warp::ws::Message;
use warp::ws::WebSocket;
use warp::Filter;

use crate::args::Args;
use crate::signaller_message::SignallerMessage;
use crate::state::StateType;

mod args;
mod config;
mod metrics;
mod peer;
mod session;
mod signaller_message;
mod state;
mod twilio_helper;

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

async fn handle_message(
    state: &mut state::State,
    tx: &Tx,
    raw_payload: &str,
    ip: IpAddr,
) -> Result<()> {
    let msg: SignallerMessage = serde_json::from_str(raw_payload)?;
    let forward_message = |state: &state::State, to: String| -> Result<()> {
        let peer = state
            .peers
            .get(&to)
            .ok_or_else(|| format_err!("Peer does not exist"))?;
        peer.sender.unbounded_send(Message::text(raw_payload))?;
        Ok(())
    };

    match msg {
        SignallerMessage::Join { from, room } => {
            match state.add_viewer(from.clone(), room.clone(), tx.clone()) {
                Ok(_) => {
                    info!("{} joined room {}", from, room);
                    forward_message(state, room)?;
                }
                Err(e) => {
                    info!("Error joining room: {}", e);
                    tx.unbounded_send(Message::text(serde_json::to_string(
                        &SignallerMessage::JoinDeclined {
                            to: from,
                            reason: e.to_string(),
                        },
                    )?))
                    .unwrap_or_else(|e| {
                        info!("Error sending failed to join response: {}", e);
                    });
                }
            };
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
            state.add_sharer(room.clone(), tx.clone(), ip)?;
            tx.unbounded_send(Message::text(serde_json::to_string(
                &SignallerMessage::StartResponse { room },
            )?))
            .unwrap_or_else(|e| {
                info!("Error sending start response: {}", e);
            });
        }
        SignallerMessage::Leave { from } => {
            info!("{} is leaving", from);
            forward_message(state, state.get_room_id_from_peer_uuid(&from)?)?;
            state.leave_session(from)?;
        }
        SignallerMessage::IceServers {} => {
            let ice_servers = state.get_ice_servers().await;
            tx.unbounded_send(Message::text(serde_json::to_string(
                &SignallerMessage::IceServersResponse { ice_servers },
            )?))
            .unwrap_or_else(|e| {
                info!("Error sending ice server response: {}", e);
            });
        }
        SignallerMessage::Offer { from: _, to }
        | SignallerMessage::Answer { from: _, to }
        | SignallerMessage::Ice { from: _, to }
        | SignallerMessage::RoomClosed { to, room: _ }
        | SignallerMessage::JoinDeclined { to, reason: _ } => {
            forward_message(state, to)?;
        }
        SignallerMessage::KeepAlive {}
        | SignallerMessage::StartResponse { .. }
        | SignallerMessage::IceServersResponse { .. } => {}
    };
    Ok(())
}

async fn process_message(
    msg: Message,
    state: StateType,
    tx: &Tx,
    ip: IpAddr,
) -> std::result::Result<(), warp::Error> {
    if !msg.is_text() {
        return Ok(());
    }

    if let Ok(s) = msg.to_str() {
        let mut locked_state = state.lock().await;
        if let Err(e) = handle_message(&mut locked_state, tx, s, ip).await {
            info!(
                "Error occurred when handling message: {}\nMessage: {}",
                e,
                msg.to_str().unwrap().to_string()
            );
        }
    }
    Ok(())
}

async fn handle_connection(args: Args, state: StateType, websocket: WebSocket, addr: IpAddr) {
    let hashed_ip = metrics::hash_ip(addr, &args.ip_hash_salt).unwrap();

    metrics::NUM_CONNECTED_CLIENTS
        .with_label_values(&[hashed_ip.as_str()])
        .inc();
    info!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    let (outgoing, incoming) = websocket.split();

    let handle_incoming =
        incoming.try_for_each(|msg| process_message(msg, state.clone(), &tx, addr));

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(handle_incoming, receive_from_others);
    future::select(handle_incoming, receive_from_others).await;

    metrics::NUM_CONNECTED_CLIENTS
        .with_label_values(&[hashed_ip.as_str()])
        .dec();
    info!("{} disconnected", &addr);
    state.lock().await.on_disconnect(&addr);
}

pub(crate) async fn start_server(addr: SocketAddrV4, args: Args, state: StateType) {
    metrics::register();

    use warp::{any, ws};
    let metrics_route = warp::path!("metrics").and_then(metrics::metrics_handler);
    let ws_route = warp::path::end()
        .and(ws())
        .and(warp_real_ip::get_forwarded_for())
        .and(any().map(move || args.clone()))
        .and(any().map(move || state.clone()))
        .map(
            |ws: ws::Ws, ip_addrs: Vec<IpAddr>, args: Args, state: StateType| {
                ws.on_upgrade(move |socket| async move {
                    handle_connection(
                        args,
                        state,
                        socket,
                        *ip_addrs.last().expect("failed to get ip"),
                    )
                    .await
                })
            },
        );

    info!("Server listening on {}", addr);
    warp::serve(metrics_route.or(ws_route)).run(addr).await;
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );
    let args = args::Args::parse();
    let address = &args.address.split(':').collect::<Vec<&str>>();
    let address = SocketAddrV4::new(
        Ipv4Addr::from_str(address[0]).unwrap(),
        address[1].parse().unwrap(),
    );

    let config = config::from_env();
    let state = state::State::new(&config);

    start_server(address, args, state).await;

    Ok(())
}
