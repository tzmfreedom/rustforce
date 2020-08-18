extern crate rustforce;

mod common;

use anyhow::Result;
use common::{create_account, delete_account, find_account, get_client, Account};
use std::collections::HashMap;
use rustforce::response::QueryResponse;

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

#[tokio::test]
async fn upsert_record() -> Result<()> {
    let original_account_name = "Hello Rust";
    let new_account_name = "Bye Rust";

    let client = get_client().await?;
    let id = create_account(&client, original_account_name).await?;

    let mut params = HashMap::new();
    params.insert("Name", new_account_name);

    client.upsert("Account", "Id", &id, params).await?;

    let record = find_account(&client, &id).await?;
    assert_eq!(new_account_name, record.name);

    delete_account(&client, &id).await?;
    Ok(())
}

#[tokio::test]
async fn check_versions() -> Result<()> {
    let client = get_client().await?;
    let versions = client.versions().await?;

    assert_ne!(0, versions.len());
    Ok(())
}

#[tokio::test]
async fn query_record() -> Result<()> {
    let account_name = "Hello Rust";

    let client = get_client().await?;
    let id = create_account(&client, account_name).await?;

    let query = format!("SELECT ID, NAME FROM ACCOUNT WHERE ID = '{}'", id);
    let query_result: QueryResponse<Account> = client.query(&query).await?;

    assert_eq!(account_name, query_result.records[0].name);

    delete_account(&client, &id).await?;
    Ok(())
}