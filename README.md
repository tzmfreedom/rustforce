## Rustforce

Salesforce Client for Rust


## Install


## Usage

```rust
use rustforce::Client;
use rustforce::response::{QueryResponse, ErrorResponse};
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

    let res: Result<QueryResponse<Account>, Vec<ErrorResponse>> = client.query("SELECT Id, Name FROM Account WHERE id = '0012K00001drfGYQAY'".to_string());
    println!("{:?}", res);
}
```

### Authentication


### Query Records

```rust
let res: Result<QueryResponse<Account>, Vec<ErrorResponse>> = client.query("SELECT Id, Name FROM Account WHERE id = '0012K00001drfGYQAY'".to_string());
```

### Create Record

```rust
let mut params = HashMap::new();
params.insert("Name", "hello rust");
let r = client.create("Account", params);
println!("{:?}", r);
```

### Update Record

```rust
let r = client.update("Account", "{sobject_id}", params);
```

### Delete Record

```rust
let r = client.destroy("Account", "{sobject_id}");
```
