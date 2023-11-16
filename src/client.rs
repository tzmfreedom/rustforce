extern crate reqwest;

use crate::errors::Error;
use crate::response::{AccessToken, BulkApiCreateResponse, BulkApiStateChangeResponse, BulkApiStatusResponse, CreateResponse, DescribeGlobalResponse, DescribeResponse, ErrorResponse, QueryResponse, SearchResponse, TokenResponse, VersionResponse};
use crate::utils::substring_before;
use regex::Regex;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::{Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

/// Represents a Salesforce Client
#[derive(Clone, Default)]
pub struct Client {
    http_client: reqwest::Client,
    client_id: Option<String>,
    client_secret: Option<String>,
    login_endpoint: String,
    instance_url: Option<String>,
    pub access_token: Option<AccessToken>,
    version: String,
}

impl Client {
    /// Creates a new client when passed a Client ID and Client Secret. These
    /// can be obtained by creating a connected app in Salesforce
    pub fn new(client_id: Option<String>, client_secret: Option<String>) -> Self {
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

    /// Set the login endpoint. This is useful if you want to connect to a
    /// Sandbox
    pub fn set_login_endpoint(&mut self, endpoint: &str) -> &mut Self {
        self.login_endpoint = endpoint.to_string();
        self
    }

    /// Set API Version
    pub fn set_version(&mut self, version: &str) -> &mut Self {
        self.version = version.to_string();
        self
    }

    pub fn set_instance_url(&mut self, instance_url: &str) -> &mut Self {
        self.instance_url = Some(instance_url.to_string());
        self
    }

    /// Set Access token if you've already obtained one via one of the OAuth2
    /// flows
    pub fn set_access_token(&mut self, access_token: &str) -> &mut Self {
        self.access_token = Some(AccessToken {
            token_type: "Bearer".to_string(),
            value: access_token.to_string(),
            issued_at: "".to_string(),
        });
        self
    }
    
    pub fn get_access_token(&mut self) -> String {
        match &self.access_token {
            Some(token) => {
                return format!("{}", token.value);
            },
            None => {
                return "".to_string();
            }
        }
    }

    /// This will fetch an access token when provided with a refresh token
    pub async fn refresh(&mut self, refresh_token: &str) -> Result<&mut Self, Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_ref().unwrap()),
            ("client_secret", self.client_secret.as_ref().unwrap()),
        ];
        let res = self
            .http_client
            .post(token_url.as_str())
            .form(&params)
            .send()
            .await?;

        if res.status().is_success() {
            let r: TokenResponse = res.json().await?;
            self.access_token = Some(AccessToken {
                value: r.access_token,
                issued_at: r.issued_at,
                token_type: "Bearer".to_string(),
            });
            self.instance_url = Some(r.instance_url);
            Ok(self)
        } else {
            let token_error = res.json().await?;
            Err(Error::TokenError(token_error))
        }
    }

    /// Login to Salesforce with username and password
    pub async fn login_with_credential(
        &mut self,
        username: String,
        password: String,
    ) -> Result<&mut Self, Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "password"),
            ("client_id", self.client_id.as_ref().unwrap()),
            ("client_secret", self.client_secret.as_ref().unwrap()),
            ("username", username.as_str()),
            ("password", password.as_str()),
        ];
        let res = self
            .http_client
            .post(token_url.as_str())
            .form(&params)
            .send()
            .await?;

        if res.status().is_success() {
            let r: TokenResponse = res.json().await?;
            self.access_token = Some(AccessToken {
                value: r.access_token,
                issued_at: r.issued_at,
                token_type: r.token_type.ok_or(Error::NotLoggedIn)?,
            });
            self.instance_url = Some(r.instance_url);
            Ok(self)
        } else {
            let error_response = res.json().await?;
            Err(Error::TokenError(error_response))
        }
    }

    pub async fn login_by_soap(&mut self, username: String, password: String) -> Result<&mut Self, Error> {
        let token_url = format!(
            "{login_endpoint}/services/Soap/u/{version}",
            login_endpoint = self.login_endpoint,
            version = self.version
        );
        let body = [
            "<se:Envelope xmlns:se='http://schemas.xmlsoap.org/soap/envelope/'>",
            "<se:Header/>",
            "<se:Body>",
            "<login xmlns='urn:partner.soap.sforce.com'>",
            format!("<username>{}</username>", username).as_str(),
            format!("<password>{}</password>", password).as_str(),
            "</login>",
            "</se:Body>",
            "</se:Envelope>",
        ]
        .join("");
        let res = self
            .http_client
            .post(token_url.as_str())
            .body(body)
            .header("Content-Type", "text/xml")
            .header("SOAPAction", "\"\"")
            .send()
            .await?;
        if res.status().is_success() {
            let body_response = res.text().await?;
            let re_access_token = Regex::new(r"<sessionId>([^<]+)</sessionId>").unwrap();
            let re_instance_url = Regex::new(r"<serverUrl>([^<]+)</serverUrl>").unwrap();
            self.access_token = Some(AccessToken {
                value: String::from(
                    re_access_token
                        .captures(body_response.as_str())
                        .unwrap()
                        .get(1)
                        .unwrap()
                        .as_str(),
                ),
                issued_at: "".to_string(),
                token_type: "Bearer".to_string(),
            });
            self.instance_url = Some(substring_before(
                re_instance_url
                    .captures(body_response.as_str())
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_str(),
                "/services/",
            ));
            Ok(self)
        } else {
            let body_response = res.text().await?;
            let re_message = Regex::new(r"<faultstring>([^<]+)</faultstring>").unwrap();
            let re_error_code = Regex::new(r"<faultcode>([^<]+)</faultcode>").unwrap();
            Err(Error::LoginError(ErrorResponse {
                message: String::from(
                    re_message
                        .captures(body_response.as_str())
                        .unwrap()
                        .get(1)
                        .unwrap()
                        .as_str(),
                ),
                error_code: String::from(
                    re_error_code
                        .captures(body_response.as_str())
                        .unwrap()
                        .get(1)
                        .unwrap()
                        .as_str(),
                ),
                fields: None,
            }))
        }
    }

    /// Query record using SOQL
    pub async fn query<T: DeserializeOwned>(&self, query: &str) -> Result<QueryResponse<T>, Error> {
        let query_url = format!("{}/query/", self.base_path());
        let params = vec![("q", query)];
        let res = self.get(query_url, params).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Query All records using SOQL
    pub async fn query_all<T: DeserializeOwned>(
        &self,
        query: &str,
    ) -> Result<QueryResponse<T>, Error> {
        let query_url = format!("{}/queryAll/", self.base_path());
        let params = vec![("q", query)];
        let res = self.get(query_url, params).await?;
        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Query More records using SOQL
    pub async fn query_more<T: DeserializeOwned>(
        &self,
        next_records_url: &str,
    ) -> Result<QueryResponse<T>, Error> {
        let query_url = format!("{}/{}", self.instance_url.as_ref().unwrap(), next_records_url);
        let res = self.get(query_url, vec![]).await?;
          if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Find records using SOSL
    pub async fn search(&self, query: &str) -> Result<SearchResponse, Error> {
        let query_url = format!("{}/search/", self.base_path());
        let params = vec![("q", query)];
        let res = self.get(query_url, params).await?;
        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Get all supported API versions
    pub async fn versions(&self) -> Result<Vec<VersionResponse>, Error> {
        let versions_url = format!(
            "{}/services/data/",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?
        );
        let res = self.get(versions_url, vec![]).await?;
        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Finds a record by ID
    pub async fn find_by_id<T: DeserializeOwned>(
        &self,
        sobject_name: &str,
        id: &str,
    ) -> Result<T, Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let res = self.get(resource_url, vec![]).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Creates an SObject
    pub async fn create<T: Serialize>(
        &self,
        sobject_name: &str,
        params: T,
    ) -> Result<CreateResponse, Error> {
        let resource_url = format!("{}/sobjects/{}", self.base_path(), sobject_name);
        let res = self.post(resource_url, params).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Updates an SObject
    pub async fn update<T: Serialize>(
        &self,
        sobject_name: &str,
        id: &str,
        params: T,
    ) -> Result<(), Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let res = self.patch(resource_url, params).await?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Upserts an SObject with key
    pub async fn upsert<T: Serialize>(
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
        let res = self.patch(resource_url, params).await?;

        if res.status().is_success() {
            match res.status() {
                StatusCode::CREATED => Ok(res.json().await?),
                _ => Ok(None),
            }
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Deletes an SObject
    pub async fn destroy(&self, sobject_name: &str, id: &str) -> Result<(), Error> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let res = self.delete(resource_url).await?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(Error::ErrorResponses(res.json().await?))
        }
    }

    /// Describes all objects
    pub async fn describe_global(&self) -> Result<DescribeGlobalResponse, Error> {
        let resource_url = format!("{}/sobjects/", self.base_path());
        let res = self.get(resource_url, vec![]).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    /// Describes specific object
    pub async fn describe(&self, sobject_name: &str) -> Result<serde_json::Value, Error> {
        let resource_url = format!("{}/sobjects/{}/describe", self.base_path(), sobject_name);
        let res = self.get(resource_url, vec![]).await?;

        if res.status().is_success() {
            Ok(serde_json::from_str(res.text().await?.as_str())?)
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    pub async fn rest_get_fulluri(&self, uri: &str) -> Result<Response, Error> {
        let resource_url = format!(
            "{}/services/apexrest/{}",
            self.instance_url.as_ref().unwrap(),
            uri
        );
        let parsed = Url::parse(&resource_url).unwrap();
        // Some ownership absurdity for string refs accessed through iterators with collect
        let hash_query: HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        let paramstrings: Vec<(String, String)> = hash_query
            .keys()
            .map(|k| (String::from(k), String::from(&hash_query[k])))
            .collect();
        let params: Vec<(&str, &str)> = paramstrings
            .iter()
            .map(|&(ref x, ref y)| (&x[..], &y[..]))
            .collect();
        let path: String = parsed.path().to_string();
        let res = self.rest_get(path, params).await?;

        if res.status().is_success() {
            Ok(res)
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    pub async fn create_job<T: Serialize>(&self, params: T) -> Result<BulkApiCreateResponse, Error> {
        let resource_url = format!("{}/jobs/ingest", self.base_path());
        let res = self.post(resource_url, params).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    pub async fn upload_csv_to_job(&self, job_id: &str, csv:  Vec<u8>) -> Result<String, Error> {
        let resource_url = format!("{}/jobs/ingest/{}/batches", self.base_path(), job_id);
        let res = self.put(resource_url, csv).await?;

        if res.status().is_success() {
            println!("{:?}", res);
            Ok(res.text().await?)
        } else {
            println!("{:?}", res);
            Err(Error::DescribeError(res.json().await?))
        }
    }
    pub async fn set_upload_state<T: Serialize>(&self, job_id: &str, params: T) -> Result<BulkApiStatusResponse, Error> {
        let resource_url = format!("{}/jobs/ingest/{}", self.base_path(), job_id);
        let res = self.patch(resource_url, params).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    pub async fn check_job_status(&self, job_id: &str) -> Result<(), Error> {
        let resource_url = format!("{}/jobs/ingest/{}/", self.base_path(), job_id);
        let res = self.get(resource_url, vec![]).await?;

        if res.status().is_success() {
            Ok(res.json().await?)
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    pub async fn rest_get(
        &self,
        path: String,
        params: Vec<(&str, &str)>,
    ) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .get(url.as_str())
            .headers(self.create_header()?)
            .query(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_post<T: Serialize>(
        &self,
        path: String,
        params: T,
    ) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .post(url.as_str())
            .headers(self.create_header()?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_patch<T: Serialize>(
        &self,
        path: String,
        params: T,
    ) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .patch(url.as_str())
            .headers(self.create_header()?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_put<T: Serialize>(&self, path: String, params: T) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .put(url.as_str())
            .headers(self.create_header()?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_delete(&self, path: String) -> Result<Response, Error> {
        let url = format!("{}{}", self.instance_url.as_ref().unwrap(), path);
        let res = self
            .http_client
            .delete(url.as_str())
            .headers(self.create_header()?)
            .send()
            .await?;
        Ok(res)
    }

    async fn get(&self, url: String, params: Vec<(&str, &str)>) -> Result<Response, Error> {
        let res = self
            .http_client
            .get(url.as_str())
            .headers(self.create_header()?)
            .query(&params)
            .send()
            .await?;
        Ok(res)
    }

    async fn post<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .post(url.as_str())
            .headers(self.create_header()?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    async fn put(&self, url: String, buffer: Vec<u8>) -> Result<Response, Error> {
        let mut headers = self.create_header()?;
        headers.insert("Content-Type", "text/csv".parse().unwrap());
        headers.insert("Accept", "application/json".parse().unwrap());
        let res = self
            .http_client
            .put(url.as_str())
            .headers(headers)
            .body(buffer)
            .send()
            .await?;
        Ok(res)
    }

    async fn patch<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        let res = self
            .http_client
            .patch(url.as_str())
            .headers(self.create_header()?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    async fn delete(&self, url: String) -> Result<Response, Error> {
        let res = self
            .http_client
            .delete(url.as_str())
            .headers(self.create_header()?)
            .send()
            .await?;
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
    use crate::{errors::Error, response::QueryResponse};
    use mockito::mock;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "PascalCase")]
    struct Account {
        id: String,
        name: String,
    }

    #[tokio::test]
    async fn login_with_credentials() -> Result<(), Error> {
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

        let mut client = super::Client::new(Some("aaa".to_string()), Some("bbb".to_string()));
        let url = &mockito::server_url();
        client.set_login_endpoint(url);
        client
            .login_with_credential("u".to_string(), "p".to_string())
            .await?;
        let token = client.access_token.unwrap();
        assert_eq!("this_is_access_token", token.value);
        assert_eq!("Bearer", token.token_type);
        assert_eq!("2019-10-01 00:00:00", token.issued_at);
        assert_eq!("https://ap.salesforce.com", client.instance_url.unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn can_get_access_token() {
        let mut client = create_test_client();
        assert_eq!("this_is_access_token", client.get_access_token());
    }

    #[tokio::test]
    async fn query() -> Result<(), Error> {
        let _m = mock(
            "GET",
            "/services/data/v44.0/query/?q=SELECT+Id%2C+Name+FROM+Account",
        )
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
        let r: QueryResponse<Account> = client.query("SELECT Id, Name FROM Account").await?;
        assert_eq!(123, r.total_size);
        assert_eq!(true, r.done);
        assert_eq!("123", r.records[0].id);
        assert_eq!("foo", r.records[0].name);

        Ok(())
    }

    #[tokio::test]
    async fn create() -> Result<(), Error> {
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
            .await?;
        assert_eq!("12345", r.id);
        assert_eq!(true, r.success);

        Ok(())
    }

    #[tokio::test]
    async fn update() -> Result<(), Error> {
        let _m = mock("PATCH", "/services/data/v44.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client();
        let r = client
            .update("Account", "123", [("Name", "foo"), ("Abc__c", "123")])
            .await;
        assert_eq!(true, r.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn upsert_201() -> Result<(), Error> {
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
            .await
            .unwrap();
        assert_eq!(true, r.is_some());
        let res = r.unwrap();
        assert_eq!("12345", res.id);
        assert_eq!(true, res.success);

        Ok(())
    }

    #[tokio::test]
    async fn upsert_204() -> Result<(), Error> {
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
            .await
            .unwrap();
        assert_eq!(true, r.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn destroy() -> Result<(), Error> {
        let _m = mock("DELETE", "/services/data/v44.0/sobjects/Account/123")
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();

        let client = create_test_client();
        let r = client.destroy("Account", "123").await?;
        println!("{:?}", r);

        Ok(())
    }

    #[tokio::test]
    async fn versions() -> Result<(), Error> {
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
        let r = client.versions().await?;
        assert_eq!("Winter '19", r[0].label);
        assert_eq!("https://ap.salesforce.com/services/data/v44.0/", r[0].url);
        assert_eq!("v44.0", r[0].version);

        Ok(())
    }

    #[tokio::test]
    async fn find_by_id() -> Result<(), Error> {
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
        let r: Account = client.find_by_id("Account", "123").await?;
        assert_eq!("foo", r.name);

        Ok(())
    }

    fn create_test_client() -> super::Client {
        let mut client = super::Client::new(Some("aaa".to_string()), Some("bbb".to_string()));
        let url = &mockito::server_url();
        client.set_instance_url(url);
        client.set_access_token("this_is_access_token");
        return client;
    }

    #[tokio::test]
    async fn test_idk(){


        #[derive(Default, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct BatchJob {
            pub object: String,
            pub content_type: String,
            pub operation: String,
            pub line_ending: String
        }

        let client = create_test_client();

        let params = BatchJob {
            operation: "Insert".to_string(),
            object: "Timecard".to_string(),
            content_type: "CSV".to_string(),
            line_ending: "LF".to_string()
        };


        client.create_job(params);
    }
}
