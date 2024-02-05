use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Listening address
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    pub(crate) address: String,
    /// Salt for hashing IP addresses
    #[arg(short, long)]
    pub(crate) ip_hash_salt: String,
}
