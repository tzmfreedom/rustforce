use rustforce::Client;
use rustforce::response::{QueryResponse, ErrorResponse};
use std::env;
use std::env::VarError;
use serde::Deserialize;
use std::collections::HashMap;

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
    client.set_login_endpoint("https://login.salesforce.com");
    let result = client.login_with_credential(username, password);
    if result.is_ok() {
//    println!("{:?}", client);
        let mut params = HashMap::new();
        params.insert("Name", "hello rust");
//    let r = client.create("Account", params);
//    println!("{:?}", r);
        let r = client.upsert("Account", "ExKey__c", "0012K00001drfGYQAY1", params);
//    let r = client.destroy("Account", "0012K00001drfGYQAY");
        println!("{:?}", r);
        let res: Result<QueryResponse<Account>, Vec<ErrorResponse>> = client.query("SELECT Id, Name FROM Account WHERE id = '0012K00001drfGYQAY'".to_string());
        println!("{:?}", res);
    } else {
        println!("{:?}", result);
    }
}
