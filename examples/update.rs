use rustforce::Client;
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

fn main() {
    let client_id = env::var("SFDC_CLIENT_ID").unwrap();
    let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(client_id, client_secret);
    let r = client.login_with_credential(username, password);

    if r.is_ok() {
        let mut params = HashMap::new();
        params.insert("Name", "hello rust");
        let res = client.update("Account", "0012K00001drfGYQAY", params);
        println!("{:?}", res);
    }
}
