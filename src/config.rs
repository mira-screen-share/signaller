use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde()]
    pub twilio_account_sid: Option<String>,

    #[serde()]
    pub twilio_auth_token: Option<String>,
}

pub fn load(path: &Path) -> Result<Config, failure::Error> {
    // create a new file if it does not exist
    if !path.exists() {
        let mut file = File::create(path)?;
        let config = toml::from_str::<Config>("")?;
        file.write_all(toml::to_string(&config)?.as_ref())?;
        return Ok(config);
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(toml::from_str(&contents)?)
}
