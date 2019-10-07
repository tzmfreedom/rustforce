extern crate reqwest;

use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::{Response, Error, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use crate::response::{AccessToken, TokenResponse, QueryResponse, ErrorResponse, CreateResponse, TokenErrorResponse, DescribeResponse, DescribeGlobalResponse, SearchResponse, VersionResponse};

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
        return Client {
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

    pub fn refresh(&mut self, refresh_token: &str) -> Result<(), TokenErrorResponse> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
        ];
        let mut res = self.http_client.post(token_url.as_str())
            .form(&params)
            .send()
            .unwrap();
        if res.status().is_success() {
            let r: TokenResponse = res.json().unwrap();
            self.access_token = Some(AccessToken {
                value: r.access_token,
                issued_at: r.issued_at,
                token_type: "Bearer".to_string(),
            });
            self.instance_url = Some(r.instance_url);
            Ok(())
        } else {
            Err(res.json().unwrap())
        }
    }

    pub fn login_with_credential(&mut self, username: String, password: String) -> Result<(), TokenErrorResponse> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = [
            ("grant_type", "password"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("username", username.as_str()),
            ("password", password.as_str()),
        ];
        let mut res = self.http_client.post(token_url.as_str())
            .form(&params)
            .send()
            .unwrap();
        if res.status().is_success() {
            let r: TokenResponse = res.json().unwrap();
            self.access_token = Some(AccessToken {
                value: r.access_token,
                issued_at: r.issued_at,
                token_type: r.token_type.unwrap(),
            });
            self.instance_url = Some(r.instance_url);
            Ok(())
        } else {
            Err(res.json().unwrap())
        }
    }

    pub fn query<T: DeserializeOwned>(&self, query: &str) -> Result<QueryResponse<T>, Vec<ErrorResponse>> {
        let query_url = format!("{}/query/", self.base_path());
        let params = vec![("q", query)];
        let mut res = self.get(query_url, params).unwrap();
        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    pub fn query_all<T: DeserializeOwned>(&self, query: &str) -> Result<QueryResponse<T>, Vec<ErrorResponse>> {
        let query_url = format!("{}/queryAll/", self.base_path());
        let params = vec![("q", query)];
        let mut res = self.get(query_url, params).unwrap();
        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    pub fn search(&self, query: &str) -> Result<SearchResponse, Vec<ErrorResponse>> {
        let query_url = format!("{}/search/", self.base_path());
        let params = vec![("q", query)];
        let mut res = self.get(query_url, params).unwrap();
        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    pub fn versions(&self) -> Result<Vec<VersionResponse>, Vec<ErrorResponse>> {
        let versions_url = format!("{}/services/data/", self.instance_url.as_ref().unwrap());
        let mut res = self.get(versions_url, vec![]).unwrap();
        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    pub fn find_by_id<T: DeserializeOwned>(&self, sobject_name: &str, id: &str) -> Result<T, Vec<ErrorResponse>> {
        let resource_url = format ! ("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let mut res = self.get(resource_url, vec! []).unwrap();

        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    pub fn create<T: Serialize>(&self, sobject_name: &str, params: T) -> Result<CreateResponse, Vec<ErrorResponse>> {
        let resource_url = format!("{}/sobjects/{}", self.base_path(), sobject_name);
        let mut res = self.post(resource_url, params).unwrap();

        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    pub fn update<T: Serialize>(&self, sobject_name: &str, id: &str, params: T) -> Result<(), Vec<ErrorResponse>> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let mut res = self.patch(resource_url, params).unwrap();

        if res.status().is_success() {
            return Ok(());
        }
        return Err(res.json().unwrap());
    }

    pub fn upsert<T: Serialize>(&self, sobject_name: &str, key_name: &str, key: &str, params: T) -> Result<Option<CreateResponse>, Vec<ErrorResponse>> {
        let resource_url = format!("{}/sobjects/{}/{}/{}", self.base_path(), sobject_name, key_name, key);
        let mut res = self.patch(resource_url, params).unwrap();

        if res.status().is_success() {
            return match res.status() {
                StatusCode::CREATED => Ok(res.json().unwrap()),
                _ => Ok(None),
            }
        }
        return Err(res.json().unwrap());
    }

    pub fn destroy(&self, sobject_name: &str, id: &str) -> Result<(), Vec<ErrorResponse>> {
        let resource_url = format!("{}/sobjects/{}/{}", self.base_path(), sobject_name, id);
        let mut res = self.delete(resource_url).unwrap();

        if res.status().is_success() {
            return Ok(());
        }
        return Err(res.json().unwrap());
    }

    pub fn describe_global(&self) -> Result<DescribeGlobalResponse, ErrorResponse> {
        let resource_url = format!("{}/sobjects/", self.base_path());
        let mut res = self.get(resource_url, vec![]).unwrap();

        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    pub fn describe(&self, sobject_name: &str) -> Result<DescribeResponse, ErrorResponse> {
        let resource_url = format!("{}/sobjects/{}/describe", self.base_path(), sobject_name);
        let mut res = self.get(resource_url, vec![]).unwrap();

        if res.status().is_success() {
            return Ok(res.json().unwrap());
        }
        return Err(res.json().unwrap());
    }

    fn get(&self, url: String, params: Vec<(&str, &str)>) -> Result<Response, Error> {
        return self.http_client.get(url.as_str())
            .headers(self.create_header())
            .query(&params)
            .send();
    }

    fn post<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        return self.http_client.post(url.as_str())
            .headers(self.create_header())
            .json(&params)
            .send();
    }

    fn patch<T: Serialize>(&self, url: String, params: T) -> Result<Response, Error> {
        return self.http_client.patch(url.as_str())
            .headers(self.create_header())
            .json(&params)
            .send();
    }

    fn delete(&self, url: String) -> Result<Response, Error> {
        return self.http_client.delete(url.as_str())
            .headers(self.create_header())
            .send();
    }

    fn create_header(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", self.access_token.as_ref().unwrap().value).parse().unwrap());
        return headers;
    }

    fn base_path(&self) -> String {
        format!("{}/services/data/{}", self.instance_url.as_ref().unwrap(), self.version)
    }
}
