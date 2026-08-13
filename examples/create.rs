use rustforce::{Client, Error, RestApi};
use std::collections::HashMap;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client_id = env::var("SFDC_CLIENT_ID").unwrap();
    let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new();
    client.set_client_id(&client_id);
    client.set_client_secret(&client_secret);
    client.login_with_credential(&username, &password).await?;

    let mut api = RestApi::new(client);

    let mut params = HashMap::new();
    params.insert("Name", "hello rust");

    let body = api.create("Account", params).await?;
    println!("{:?}", body);

    Ok(())
}
