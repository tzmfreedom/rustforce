use rustforce::Client;
use std::env;

fn main() {
    let client_id = env::var("SFDC_CLIENT_ID").unwrap();
    let client_secret = env::var("SFDC_CLIENT_SECRET").unwrap();
    let username = env::var("SFDC_USERNAME").unwrap();
    let password = env::var("SFDC_PASSWORD").unwrap();

    let mut client = Client::new(client_id, client_secret);
    let r = client.login_with_credential(username, password);

    if r.is_ok() {
        //        let res = client.describe("Account");
        let res = client.describe_global();
        println!("{:?}", res);
    }
}
