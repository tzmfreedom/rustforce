use rustforce::{Client, Error, ToolingApi};
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

    let mut tooling = ToolingApi::new(client);

    // Get current user ID
    let user_info = tooling.get_current_user_id().await?;
    let user_id = user_info["id"].as_str().unwrap();
    println!("Current user: {} ({})", user_info["displayName"], user_id);

    // Find or create a debug level
    let debug_levels = tooling.get_debug_level("SFDataloaderDebug").await?;
    let debug_level_id = if debug_levels["totalSize"].as_i64().unwrap_or(0) > 0 {
        debug_levels["records"][0]["Id"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        let mut params = HashMap::new();
        params.insert("DeveloperName", "SFDataloaderDebug");
        params.insert("MasterLabel", "SFDataloaderDebug");
        params.insert("ApexCode", "FINEST");
        params.insert("Visualforce", "NONE");
        let result = tooling.create_debug_level(params).await?;
        result["id"].as_str().unwrap().to_string()
    };
    println!("Debug level ID: {}", debug_level_id);

    // Check for existing trace flags
    let flags = tooling.get_trace_flags(user_id).await?;
    println!("Existing trace flags: {}", flags["totalSize"]);

    // Create a trace flag
    let mut params = HashMap::new();
    params.insert("TracedEntityId", user_id);
    params.insert("DebugLevelId", debug_level_id.as_str());
    params.insert("LogType", "DEVELOPER_LOG");
    params.insert("ExpirationDate", "2026-12-31T23:59:59.000+0000");
    let result = tooling.create_trace_flag(params).await?;
    let flag_id = result["id"].as_str().unwrap();
    println!("Created trace flag: {}", flag_id);

    // Delete the trace flag when done
    tooling.delete_trace_flag(flag_id).await?;
    println!("Deleted trace flag: {}", flag_id);

    Ok(())
}
