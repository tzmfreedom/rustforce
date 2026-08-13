[![crate-name at crates.io](https://img.shields.io/crates/v/rustforce.svg)](https://crates.io/crates/rustforce)
[![crate-name at docs.rs](https://docs.rs/rustforce/badge.svg)](https://docs.rs/rustforce)

## Rustforce

Salesforce API Client for Rust

## Usage

```rust
use rustforce::{Client, RestApi, Error, QueryResponse};
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Account {
    id: String,
    name: String,
}

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

    let mut api = RestApi::new(client);

    let res: QueryResponse<Account> = api.query("SELECT Id, Name FROM Account").await?;
    println!("{:?}", res);

    Ok(())
}
```

### Authentication

#### Username Password Flow (OAuth2)

```rust
let mut client = Client::new();
client.set_client_id(&client_id);
client.set_client_secret(&client_secret);
client.login_with_credential(&username, &password).await?;
```

#### SOAP Login

```rust
let mut client = Client::new();
client.login_by_soap(&username, &password).await?;
```

#### Using an Existing Access Token

```rust
let mut client = Client::new();
client.set_instance_url("https://na1.salesforce.com");
client.set_access_token(token, issued_at, token_type);
```

### Refresh Token

```rust
client.set_refresh_token("your_refresh_token");
client.refresh().await?;
```

### REST API

All REST API methods are accessed through `RestApi`:

```rust
let mut api = RestApi::new(client);
```

### Query Records

```rust
let res: QueryResponse<Account> = api.query("SELECT Id, Name FROM Account").await?;
```

### Query All Records

```rust
let res: QueryResponse<Account> = api.query_all("SELECT Id, Name FROM Account").await?;
```

### Find By Id

```rust
let account: Account = api.find_by_id("Account", "{sf_id}").await?;
```

### Create Record

```rust
use rustforce::CreateResponse;

let mut params = HashMap::new();
params.insert("Name", "hello rust");
let res: CreateResponse = api.create("Account", params).await?;
println!("{:?}", res);
```

### Update Record

```rust
api.update("Account", "{sobject_id}", params).await?;
```

### Upsert Record

```rust
let res = api.upsert("Account", "{external_key_name}", "{external_key}", params).await?;
println!("{:?}", res.status()); // 200 = updated, 201 = created
```

### Delete Record

```rust
api.destroy("Account", "{sobject_id}").await?;
```

### Describe Global

```rust
use rustforce::DescribeGlobalResponse;

let res: DescribeGlobalResponse = api.describe_global().await?;
```

### Describe SObject

```rust
use rustforce::DescribeResponse;

let res: DescribeResponse = api.describe("Account").await?;
```

### Versions

```rust
use rustforce::VersionResponse;

let versions: Vec<VersionResponse> = api.versions().await?;
```

### Search (SOSL)

```rust
use rustforce::SearchResponse;

let res: SearchResponse = api.search_sosl("FIND {Rust}").await?;
```

### Bulk API v1

```rust
use rustforce::BulkApi;

let mut bulk = BulkApi::new(client);
let res = bulk.create_job(params).await?;
```

### Bulk API v2

```rust
use rustforce::BulkApiV2;

let mut bulk = BulkApiV2::new(client);
let res = bulk.create_job(params).await?;
```
