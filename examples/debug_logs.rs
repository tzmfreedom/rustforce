use rustforce::{Client, Error, ToolingApi};
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

    let mut tooling = ToolingApi::new(client);

    // Get the most recent log ID
    let logs = tooling.get_latest_apex_logs(1).await?;
    println!("Latest logs: {:?}", logs);

    if let Some(records) = logs["records"].as_array() {
        if let Some(log) = records.first() {
            let log_id = log["Id"].as_str().unwrap();
            // Fetch the full log body
            let body = tooling.get_apex_log_body(log_id).await?;
            println!("Log body:\n{}", body);
        }
    }

    Ok(())
}
