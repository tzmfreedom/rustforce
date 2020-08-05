extern crate reqwest;

use crate::errors::Error;
use crate::response::{
    AccessToken, CreateResponse, DescribeGlobalResponse, DescribeResponse, QueryResponse,
    SearchResponse, TokenResponse, VersionResponse,
};
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Client {
    http_client: reqwest::Client,
    client_id: String,
    client_secret: String,
    login_endpoint: String,
    instance_url: Option<String>,
    access_token: Option<AccessToken>,
    version: String,
}

impl Client {
    pub fn new(client_id: String, client_secret: String) -> Client {
        let http_client = reqwest::Client::new();
        Client {
            http_client,
            client_id,
            client_secret,
            login_endpoint: "https://login.salesforce.com".to_string(),
            access_token: None,
            instance_url: None,
            version: "v44.0".to_string(),
        }
    }

    pub fn set_login_endpoint(&mut self, endpoint: &str) {
        self.login_endpoint = endpoint.to_string();
    }

    pub fn set_version(&mut self, version: &str) {
        self.version = version.to_string();
    }

    pub fn set_instance_url(&mut self, instance_url: &str) {
        self.instance_url = Some(instance_url.to_string());
    }

    pub fn set_access_token(&mut self, access_token: &str) {
        self.access_token = Some(AccessToken {
            token_type: "Bearer".to_string(),
            value: access_token.to_string(),
            issued_at: "".to_string(),
        });
    }

    pub fn refresh(&mut self, refresh_token: &str) -> Result<(), Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
        ];
        let mut res = self
            .http_client
            .post(token_url.as_str())
            .form(&params)
            .send()?;

        if res.status().is_success() {
            let r: TokenResponse = res.json()?;
            self.access_token = Some(AccessToken {
                value: r.access_token,
                issued_at: r.issued_at,
                token_type: "Bearer".to_string(),
            });
            self.instance_url = Some(r.instance_url);
            Ok(())
        } else {
            let token_error = res.json()?;
            Err(Error::TokenError(token_error))
        }
    }

    pub fn login_with_credential(
        &mut self,
        username: String,
        password: String,
    ) -> Result<(), Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "password"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("username", username.as_str()),
            ("password", password.as_str()),
        ];
        let mut res = self
            .http_client
            .post(token_url.as_str())
            .form(&params)
            .send()?;

        if res.status().is_success() {
            let r: TokenResponse = res.json()?;
            self.access_token = Some(AccessToken {
                value: r.access_token,
                issued_at: r.issued_at,
                token_type: r.token_type.ok_or(Error::NotLoggedIn)?,
            });
            self.instance_url = Some(r.instance_url);
            Ok(())
        } else {
            let error_response = res.json()?;
            Err(Error::TokenError(error_response))
        }
    }

    pub fn query<T: DeserializeOwned>(&self, query: &str) -> Result<QueryResponse<T>, Error> {
        let query_url = format!("{}/query/", self.base_path());
        let params = vec![("q", query)];
        let mut res = self.get(query_url, params)?;
        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn query_all<T: DeserializeOwned>(&self, query: &str) -> Result<QueryResponse<T>, Error> {
        let query_url = format!("{}/queryAll/", self.base_path());
        let params = vec![("q", query)];
        let mut res = self.get(query_url, params)?;
        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn search(&self, query: &str) -> Result<SearchResponse, Error> {
        let query_url = format!("{}/search/", self.base_path());
        let params = vec![("q", query)];
        let mut res = self.get(query_url, params)?;
        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn versions(&self) -> Result<Vec<VersionResponse>, Error> {
        let versions_url = format!(
            "{}/services/data/",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?
        );
        let mut res = self.get(versions_url, vec![])?;
        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn find_by_id<T: DeserializeOwned>(
        &self,
        sobject_name: &str,
        id: &str,
    ) -> Result<T, Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let mut res = self.get(resource_url, vec![])?;

        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn create<T: Serialize>(
        &self,
        sobject_name: &str,
        params: T,
    ) -> Result<CreateResponse, Error> {
        let resource_url = format!("{}/sobjects/{}", self.base_path(), sobject_name);
        let mut res = self.post(resource_url, params)?;

        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn update<T: Serialize>(
        &self,
        sobject_name: &str,
        id: &str,
        params: T,
    ) -> Result<(), Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let mut res = self.patch(resource_url, params)?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn upsert<T: Serialize>(
        &self,
        sobject_name: &str,
        key_name: &str,
        key: &str,
        params: T,
    ) -> Result<Option<CreateResponse>, Error> {
        let resource_url = format!(
            "{}/sobjects/{}/{}/{}",
            self.base_path(),
            sobject_name,
            key_name,
            key
        );
        let mut res = self.patch(resource_url, params)?;

        if res.status().is_success() {
            match res.status() {
                StatusCode::CREATED => Ok(res.json()?),
                _ => Ok(None),
            }
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn destroy(&self, sobject_name: &str, id: &str) -> Result<(), Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let mut res = self.delete(resource_url)?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(Error::ErrorResponses(res.json()?))
        }
    }

    pub fn describe_global(&self) -> Result<DescribeGlobalResponse, Error> {
        let resource_url = format!("{}/sobjects/", self.base_path());
        let mut res = self.get(resource_url, vec![])?;

        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::DescribeError(res.json()?))
        }
    }

    pub fn describe(&self, sobject_name: &str) -> Result<DescribeResponse, Error> {
        let resource_url = format!("{}/sobjects/{}/describe", self.base_path(), sobject_name);
        let mut res = self.get(resource_url, vec![])?;

        if res.status().is_success() {
            Ok(res.json()?)
        } else {
            Err(Error::DescribeError(res.json()?))
        }
    }

    fn get(&self, url: String, params: Vec<(&str, &str)>) -> Result<Response, Error> {
        let res = self
            .http_client
            .get(url.as_str())
            .headers(self.create_header()?)
            .query(&params)
            .send()?;
        Ok(res)
    }

    fn post<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .post(url.as_str())
            .headers(self.create_header()?)
            .json(&params)
            .send()?;
        Ok(res)
    }

    fn patch<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .patch(url.as_str())
            .headers(self.create_header()?)
            .json(&params)
            .send()?;
        Ok(res)
    }

    fn delete(&self, url: String) -> Result<Response, Error> {
        let res = self
            .http_client
            .delete(url.as_str())
            .headers(self.create_header()?)
            .send()?;
        Ok(res)
    }

    fn create_header(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!(
                "Bearer {}",
                self.access_token.as_ref().ok_or(Error::NotLoggedIn)?.value
            )
            .parse()?,
        );

        Ok(headers)
    }

    fn base_path(&self) -> String {
        format!(
            "{}/services/data/{}",
            self.instance_url.as_ref().unwrap(),
            self.version
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::response::{QueryResponse};
    use mockito::{mock};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct Account {
        id: String,
        name: String,
    }

    #[test]
    fn login_with_credentials() {
        let _m = mock("POST", "/services/oauth2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "this_is_access_token",
                    "issued_at": "2019-10-01 00:00:00",
                    "id": "12345",
                    "instance_url": "https://ap.salesforce.com",
                    "signature": "abcde",
                    "token_type": "Bearer",
                })
                .to_string(),
            )
            .create();

        let mut client = super::Client::new("aaa".to_string(), "bbb".to_string());
        let url = &mockito::server_url();
        client.set_login_endpoint(url);
        let r = client.login_with_credential("u".to_string(), "p".to_string());
        let token = client.access_token.unwrap();
        assert_eq!(true, r.is_ok());
        assert_eq!("this_is_access_token", token.value);
        assert_eq!("Bearer", token.token_type);
        assert_eq!("2019-10-01 00:00:00", token.issued_at);
        assert_eq!("https://ap.salesforce.com", client.instance_url.unwrap());
    }

    #[test]
    fn query() {
        let _m = mock("GET", "/services/data/v44.0/query/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "totalSize": 123,
                    "done": true,
                    "records": vec![
                        Account {
                            id: "123".to_string(),
                            name: "foo".to_string(),
                        },
                    ]
                })
                .to_string(),
            )
            .create();

        let client = create_test_client();
        let r: QueryResponse<Account> = client.query("SELECT Id, Name FROM Account").unwrap();
        assert_eq!(123, r.total_size);
        assert_eq!(true, r.done);
        assert_eq!("123", r.records[0].id);
        assert_eq!("foo", r.records[0].name);
    }

    #[test]
    fn create() {
        let _m = mock("POST", "/services/data/v44.0/sobjects/Account")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                                "id": "12345",
                                "success": true,
                //                "errors": vec![],
                            })
                .to_string(),
            )
            .create();

        let client = create_test_client();
        let r = client
            .create("Account", [("Name", "foo"), ("Abc__c", "123")])
            .unwrap();
        assert_eq!("12345", r.id);
        assert_eq!(true, r.success);
    }

    #[test]
    fn update() {
        let _m = mock("PATCH", "/services/data/v44.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client();
        let r = client.update("Account", "123", [("Name", "foo"), ("Abc__c", "123")]);
        assert_eq!(true, r.is_ok());
    }

    #[test]
    fn upsert_201() {
        let _m = mock(
            "PATCH",
            "/services/data/v44.0/sobjects/Account/ExKey__c/123",
        )
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                            "id": "12345",
                            "success": true,
            //                "errors": vec![],
                        })
            .to_string(),
        )
        .create();

        let client = create_test_client();
        let r = client
            .upsert(
                "Account",
                "ExKey__c",
                "123",
                [("Name", "foo"), ("Abc__c", "123")],
            )
            .unwrap();
        assert_eq!(true, r.is_some());
        let res = r.unwrap();
        assert_eq!("12345", res.id);
        assert_eq!(true, res.success);
    }

    #[test]
    fn upsert_204() {
        let _m = mock(
            "PATCH",
            "/services/data/v44.0/sobjects/Account/ExKey__c/123",
        )
        .with_status(204)
        .with_header("content-type", "application/json")
        .create();

        let client = create_test_client();
        let r = client
            .upsert(
                "Account",
                "ExKey__c",
                "123",
                [("Name", "foo"), ("Abc__c", "123")],
            )
            .unwrap();
        assert_eq!(true, r.is_none());
    }

    #[test]
    fn destroy() {
        let _m = mock("DELETE", "/services/data/v44.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client();
        let r = client.destroy("Account", "123");
        println!("{:?}", r);
        assert_eq!(true, r.is_ok());
    }

    #[test]
    fn versions() {
        let _m = mock("GET", "/services/data/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!([{
                    "label": "Winter '19",
                    "url": "https://ap.salesforce.com/services/data/v44.0/",
                    "version": "v44.0",
                }])
                .to_string(),
            )
            .create();

        let client = create_test_client();
        let r = client.versions().unwrap();
        assert_eq!("Winter '19", r[0].label);
        assert_eq!("https://ap.salesforce.com/services/data/v44.0/", r[0].url);
        assert_eq!("v44.0", r[0].version);
    }

    #[test]
    fn find_by_id() {
        let _m = mock("GET", "/services/data/v44.0/sobjects/Account/123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "Id": "123",
                    "Name": "foo",
                })
                .to_string(),
            )
            .create();

        let client = create_test_client();
        let r: Account = client.find_by_id("Account", "123").unwrap();
        assert_eq!("foo", r.name);
    }

    fn create_test_client() -> super::Client {
        let mut client = super::Client::new("aaa".to_string(), "bbb".to_string());
        let url = &mockito::server_url();
        client.set_instance_url(url);
        client.set_access_token("this_is_access_token");
        return client;
    }
}
