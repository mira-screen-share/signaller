use std::net::SocketAddrV4;

use lazy_static::lazy_static;
use log::{error, info};
use prometheus::{IntGauge, Registry};
use warp::{Filter, Rejection, Reply};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref CONNECTED_CLIENTS: IntGauge =
        IntGauge::new("connected_clients", "Connected Clients").expect("metric can be created");
}

fn register() {
    REGISTRY
        .register(Box::new(CONNECTED_CLIENTS.clone()))
        .expect("collector can be registered");
}

pub(crate) async fn start_server(addr: SocketAddrV4) {
    register();
    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    info!("Metrics server listening on {}", addr);
    warp::serve(metrics_route).run(addr).await;
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        error!("could not encode custom metrics: {}", e);
    };
    let mut res = String::from_utf8(buffer.clone()).unwrap_or_else(|e| {
        error!("custom metrics could not be from_utf8'd: {}", e);
        String::default()
    });
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        error!("could not encode prometheus metrics: {}", e);
    };
    let res_custom = String::from_utf8(buffer.clone()).unwrap_or_else(|e| {
        error!("prometheus metrics could not be from_utf8'd: {}", e);
        String::default()
    });
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}
