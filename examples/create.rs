use rustforce::{Client, Error};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Account {
    #[serde(rename = "attributes")]
    attributes: Attribute,
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Attribute {
    url: String,
    #[serde(rename = "type")]
    sobject_type: String,
}

fn main() -> Result<(), Error> {
    let client_id = env::var("SFDC_CLIENT_ID").unwrap();
    let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(client_id, client_secret);
    client.login_with_credential(username, password)?;

    let mut params = HashMap::new();
    params.insert("Name", "hello rust");

    let res = client.create("Account", params)?;
    println!("{:?}", res);

    Ok(())
}
