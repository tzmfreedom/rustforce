use anyhow::Result;
use rustforce::Client;
use serde::Deserialize;
use std::{collections::HashMap, env};

struct Credentials {
    client_id: String,
    client_secret: String,
    username: String,
    password: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
    #[serde(rename = "attributes")]
    pub attributes: Attribute,
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Attribute {
    pub url: String,
    #[serde(rename = "type")]
    pub sobject_type: String,
}

fn get_credentials() -> Result<Credentials> {
    Ok(Credentials {
        client_id: env::var("SFDC_CLIENT_ID")?,
        client_secret: env::var("SFDC_CLIENT_SECRET")?,
        username: env::var("SFDC_USERNAME")?,
        password: env::var("SFDC_PASSWORD")?,
    })
}

pub async fn get_client() -> Result<Client> {
    let creds = get_credentials()?;

    let mut client = Client::new(creds.client_id, creds.client_secret);
    client
        .login_with_credential(creds.username, creds.password)
        .await?;

    Ok(client)
}

pub async fn create_account(client: &Client, name: &str) -> Result<String> {
    let mut params = HashMap::new();
    params.insert("Name", name);

    let res = client.create("Account", params).await?;

    Ok(res.id)
}

pub async fn delete_account(client: &Client, id: &str) -> Result<()> {
    client.destroy("Account", &id).await?;
    Ok(())
}

pub async fn find_account(client: &Client, id: &str) -> Result<Account> {
    let res: Account = client.find_by_id("Account", id).await?;
    Ok(res)
}
