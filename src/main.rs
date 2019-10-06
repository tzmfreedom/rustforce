use rustforce::client::{Client, QueryResponse};
use std::env;
use std::env::VarError;
use serde::Deserialize;

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
    client.login_with_credential(username, password);
//    println!("{:?}", client);
    client.create("Account", [("name", "create from rust")]);
    let res: QueryResponse<Account> = client.query("SELECT Id, Name FROM Account ORDER BY CreatedDate DESC LIMIT 1".to_string());
    println!("{:?}", res);
}
