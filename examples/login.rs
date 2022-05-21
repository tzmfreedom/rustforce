use rustforce::{Client, Error};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(None, None);
    client.login_by_soap(username, password).await?;
    Ok(())
}
