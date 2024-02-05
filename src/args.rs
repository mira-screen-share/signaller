use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Listening Websocket address
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    pub(crate) address: String,
    /// Metrics server address
    #[arg(short, long, default_value = "0.0.0.0:8081")]
    pub(crate) metrics_address: String,
}
