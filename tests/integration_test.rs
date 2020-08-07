extern crate rustforce;

mod common;

use anyhow::Result;
use common::{create_account, delete_account, find_account, get_client};
use std::collections::HashMap;

#[tokio::test]
async fn create_find_delete_record() -> Result<()> {
    let account_name = "Hello Rust";
    let client = get_client().await?;
    let id = create_account(&client, account_name).await?;
    assert_ne!(String::new(), id);

    let record = find_account(&client, &id).await?;

    assert_eq!(account_name, record.name);
    delete_account(&client, &id).await?;

    Ok(())
}

#[tokio::test]
async fn update_record() -> Result<()> {
    let new_account_name = "Bye Rust";

    let client = get_client().await?;
    let id = create_account(&client, "Hello Rust").await?;

    let mut params = HashMap::new();
    params.insert("Name", new_account_name);

    client.update("Account", &id, params).await?;

    let record = find_account(&client, &id).await?;
    assert_eq!(new_account_name, record.name);

    delete_account(&client, &id).await?;
    Ok(())
}
