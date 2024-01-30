use log::error;
use serde_json::Value;

use crate::signaller_message::IceServer;

pub async fn get_twilio_ice_servers(
    client: &twilio::TwilioClient,
    account_sid: &String,
) -> Vec<IceServer> {
    let response = client.create_token(account_sid.as_str()).send().await;
    match response {
        Ok(token) => token
            .ice_servers
            .unwrap_or_default()
            .iter()
            .map(|s| match s {
                Value::Object(s) => {
                    let url = s.get("url").unwrap().as_str().unwrap().to_owned();
                    IceServer {
                        url,
                        username: token.username.clone().unwrap(),
                        password: token.password.clone().unwrap(),
                    }
                }
                _ => panic!("Expected object"),
            })
            .collect(),
        Err(e) => {
            error!("Failed to get Twilio ICE servers: {:?}", e);
            vec![]
        }
    }
}
