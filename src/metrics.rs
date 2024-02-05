use lazy_static::lazy_static;
use log::error;
use prometheus::{Histogram, HistogramOpts, IntGauge, IntGaugeVec, Opts, Registry};
use warp::{Rejection, Reply};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    pub static ref NUM_CONNECTED_CLIENTS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("num_connected_clients", "Connected Clients"),
        &["hashed_ip"]
    )
    .expect("metric can be created");
    pub static ref NUM_ONGOING_SESSIONS: IntGauge =
        IntGauge::new("num_ongoing_sessions", "Ongoing Sessions").expect("metric can be created");
    pub static ref SESSION_DURATION_SEC: Histogram = Histogram::with_opts(
        HistogramOpts::new("session_duration_sec", "Session Duration Seconds").buckets(vec![
            1.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0, 60.0, 90.0, 120.0, 180.0, 240.0,
            300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 14400.0, 28800.0, 43200.0, 86400.0,
        ])
    )
    .expect("metric can be created");
}

pub(crate) fn register() {
    REGISTRY
        .register(Box::new(NUM_CONNECTED_CLIENTS.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(NUM_ONGOING_SESSIONS.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(SESSION_DURATION_SEC.clone()))
        .expect("collector can be registered");
}

pub(crate) async fn metrics_handler() -> Result<impl Reply, Rejection> {
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

pub fn hash_ip(ip: std::net::IpAddr, salt: &str) -> Result<String, argon2::password_hash::Error> {
    use argon2::{
        password_hash::{PasswordHasher, SaltString},
        Argon2,
    };
    Ok(Argon2::default()
        .hash_password(ip.to_string().as_bytes(), &SaltString::from_b64(salt)?)?
        .hash
        .unwrap()
        .to_string())
}
