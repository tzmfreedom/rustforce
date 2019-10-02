use std::collections::HashMap;
use rustforce::client::Client;
use std::env;
use std::env::VarError;

fn main() -> Result<(), VarError> {
    let client_id = env::var("SFDC_CLIENT_ID")?;
    let client_secret = env::var("SFDC_CLIENT_SECRET")?;
    let username = env::var("SFDC_USERNAME")?;
    let password = env::var("SFDC_PASSWORD")?;

    let mut client = Client::new(client_id.as_str(), client_secret.as_str());
    client.loginWithCredential(username.as_str(), password.as_str());
    Ok(())
}
