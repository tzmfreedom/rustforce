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

    let result = tooling
        .execute_anonymous("System.debug('Hello from Rust!');")
        .await?;

    if result.success {
        println!("Apex executed successfully!");
    } else if !result.compiled {
        println!(
            "Compile error at line {} column {}: {}",
            result.line,
            result.column,
            result.compile_problem.unwrap_or_default()
        );
    } else {
        println!(
            "Runtime error: {}",
            result.exception_message.unwrap_or_default()
        );
        if let Some(trace) = result.exception_stack_trace {
            println!("Stack trace: {}", trace);
        }
    }

    Ok(())
}
