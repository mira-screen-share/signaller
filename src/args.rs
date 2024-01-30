use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Listening Websocket address
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    pub(crate) address: String,

    /// Config file path
    #[arg(short, long, default_value = "config.toml")]
    pub(crate) config: String,
}
