use crate::access_token::AccessToken;
use crate::errors::Error;
use crate::responses::error_response::ErrorResponse;
use crate::responses::token_response::TokenResponse;
use crate::xml::{extract_xml_tag, create_login_envelope};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::{Response, Url};
use serde::Serialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct Client {
    pub(crate) http_client: reqwest::Client,
    pub(crate) client_id: Option<String>,
    pub(crate) client_secret: Option<String>,
    pub(crate) login_endpoint: String,
    pub(crate) instance_url: Option<String>,
    pub(crate) access_token: Option<AccessToken>,
    pub(crate) refresh_token: Option<String>,
    pub(crate) version: String,
    pub(crate) secret_required: bool,
}

impl Default for Client {
    fn default() -> Self {
        Client::new()
    }
}

impl Client {
    pub fn new() -> Client {
        let http_client = reqwest::Client::new();
        Client {
            http_client,
            client_id: None,
            client_secret: None,
            login_endpoint: "https://login.salesforce.com".to_string(),
            access_token: None,
            instance_url: None,
            refresh_token: None,
            secret_required: true,
            version: "v60.0".to_string(),
        }
    }

    // --- Read-only getters ---

    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
    }

    pub fn login_endpoint(&self) -> &str {
        &self.login_endpoint
    }

    pub fn instance_url(&self) -> Option<&str> {
        self.instance_url.as_deref()
    }

    pub fn access_token(&self) -> Option<&AccessToken> {
        self.access_token.as_ref()
    }

    pub fn access_token_value(&self) -> Option<&str> {
        self.access_token.as_ref().map(|t| t.value.as_str())
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    pub fn base_path(&self) -> Result<String, Error> {
        let instance_url = self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?;
        Ok(format!("{}/services/data/{}", instance_url, self.version))
    }

    // --- Setters ---

    pub fn set_login_endpoint(&mut self, endpoint: &str) -> &mut Self {
        self.login_endpoint = endpoint.to_string();
        self
    }

    pub fn set_version(&mut self, version: &str) -> &mut Self {
        self.version = version.to_string();
        self
    }

    pub fn set_instance_url(&mut self, instance_url: &str) -> &mut Self {
        self.instance_url = Some(instance_url.to_string());
        self
    }

    pub fn set_refresh_token(&mut self, refresh_token: &str) -> &mut Self {
        self.refresh_token = Some(refresh_token.to_string());
        self
    }

    pub fn set_secret_required(&mut self, secret_required: bool) -> &mut Self {
        self.secret_required = secret_required;
        self
    }

    pub fn set_client_id(&mut self, client_id: &str) -> &mut Self {
        self.client_id = Some(client_id.to_string());
        self
    }

    pub fn set_client_secret(&mut self, client_secret: &str) -> &mut Self {
        self.client_secret = Some(client_secret.to_string());
        self
    }

    /// Set Access token if you've already obtained one via one of the OAuth2 flows
    pub fn set_access_token(
        &mut self,
        access_token: String,
        issued_at: String,
        token_type: String,
    ) -> &mut Self {
        self.access_token = Some(AccessToken {
            token_type,
            value: access_token,
            issued_at,
        });
        self
    }

    pub async fn get_identity(&mut self, identity_url: String) -> Result<String, Error> {
        let res = self.get(identity_url, vec![]).await?;
        if res.status().is_success() {
            Ok(res.text().await?)
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    pub async fn ensure_refresh(&mut self) -> Result<&mut Self, Error> {
        if self.access_token.is_none() {
            return Ok(self);
        }

        let issued_at = &self.access_token.as_ref().unwrap().issued_at;
        let timestamp_ms = match issued_at.parse::<u64>() {
            Ok(ts) => ts,
            Err(_) => {
                // SOAP login returns ISO timestamp; can't compare, attempt refresh
                log::info!("Could not parse issued_at as timestamp, attempting refresh.");
                return self.refresh().await;
            }
        };
        let seconds = timestamp_ms / 1000;
        let nanos = (timestamp_ms % 1000) * 1_000_000;

        let given_time = UNIX_EPOCH + Duration::new(seconds, nanos as u32);

        let two_hours = Duration::from_secs(2 * 60 * 60); // 2 hours in seconds
        let modified_time = given_time + two_hours;

        let current_time = SystemTime::now();

        if current_time > modified_time {
            log::info!("Access Token Expired, Refreshing.");
            Ok(self.refresh().await?)
        } else {
            Ok(self)
        }
    }

    fn get_refresh_params(&self) -> Vec<(String, String)> {
        let refresh_token = self.refresh_token.clone().unwrap_or_default();
        let client_id = self.client_id.clone().unwrap_or_default();

        let mut params = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("refresh_token".to_string(), refresh_token),
            ("client_id".to_string(), client_id),
        ];

        if self.secret_required {
            params.push((
                "client_secret".to_string(),
                self.client_secret.clone().unwrap_or_default(),
            ));
        }
        params
    }

    /// This will fetch an access token when provided with a refresh token
    pub async fn refresh(&mut self) -> Result<&mut Self, Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let params = self.get_refresh_params();

        let res = self
            .http_client
            .post(token_url.as_str())
            .form(&params)
            .send()
            .await;

        let res = match res {
            Ok(res) => res,
            Err(e) => return Err(e.into()),
        };

        if !res.status().is_success() {
            let error_response = res.json().await?;
            return Err(Error::TokenError(error_response));
        }

        let response: TokenResponse = res.json().await?;
        let token_type = response.token_type.unwrap_or_default();
        self.set_access_token(response.access_token, response.issued_at, token_type);
        self.instance_url = Some(response.instance_url);
        Ok(self)
    }

    /// Login to Salesforce with username and password
    pub async fn login_with_credential(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<&mut Self, Error> {
        let token_url = format!("{}/services/oauth2/token", self.login_endpoint);
        let client_id = self
            .client_id
            .as_ref()
            .ok_or_else(|| Error::ConfigError("client_id is required".to_string()))?;
        let client_secret = self
            .client_secret
            .as_ref()
            .ok_or_else(|| Error::ConfigError("client_secret is required".to_string()))?;
        let params = [
            ("grant_type", "password"),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("username", username),
            ("password", password),
        ];
        let res = self
            .http_client
            .post(token_url.as_str())
            .form(&params)
            .send()
            .await?;

        if !(res.status().is_success()) {
            let error_response = res.json().await?;
            return Err(Error::TokenError(error_response));
        }

        let response: TokenResponse = res.json().await?;
        let token_type = response.token_type.unwrap_or_default();
        self.set_access_token(response.access_token, response.issued_at, token_type);
        self.instance_url = Some(response.instance_url);
        Ok(self)
    }

    pub async fn login_by_soap(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<&mut Self, Error> {
        let token_url = format!("{}/services/Soap/u/{}", self.login_endpoint, self.version);
        let body = create_login_envelope(username, password);
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
            self.access_token = match extract_xml_tag("sessionId", body_response.as_str()) {
                Some(t) => {
                    let issued_at =
                        extract_xml_tag("serverTimestamp", body_response.as_str())
                            .unwrap_or_default();
                    Some(AccessToken {
                        value: t,
                        issued_at,
                        token_type: "Bearer".to_string(),
                    })
                }
                None => None,
            };
            self.instance_url = extract_xml_tag("serverUrl", body_response.as_str());
            Ok(self)
        } else {
            let body_response = res.text().await?;
            let error_message =
                extract_xml_tag("faultstring", body_response.as_str()).unwrap_or_default();
            let error_code =
                extract_xml_tag("faultcode", body_response.as_str()).unwrap_or_default();

            Err(Error::LoginError(ErrorResponse {
                message: error_message,
                error_code,
                fields: None,
            }))
        }
    }

    pub async fn rest_get_fulluri(&mut self, uri: &str) -> Result<Response, Error> {
        let resource_url = format!(
            "{}/services/apexrest/{}",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?,
            uri
        );
        let parsed = Url::parse(&resource_url)
            .map_err(|e| Error::ConfigError(format!("Invalid URL: {}", e)))?;
        // Some ownership absurdity for string refs accessed through iterators with collect
        let hash_query: HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        let params_string: Vec<(String, String)> = hash_query
            .keys()
            .map(|k| (String::from(k), String::from(&hash_query[k])))
            .collect();
        let params: Vec<(&str, &str)> = params_string
            .iter()
            .map(|(x, y)| (&x[..], &y[..]))
            .collect();
        let path: String = parsed.path().to_string();
        self.rest_get(path, params).await
    }

    pub async fn rest_get(
        &mut self,
        path: String,
        params: Vec<(&str, &str)>,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let url = format!(
            "{}{}",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?,
            path
        );
        let res = self
            .http_client
            .get(url.as_str())
            .headers(self.create_header(vec![])?)
            .query(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_post<T: Serialize>(
        &mut self,
        path: String,
        params: T,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let url = format!(
            "{}{}",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?,
            path
        );
        let res = self
            .http_client
            .post(url)
            .headers(self.create_header(vec![])?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_patch<T: Serialize>(
        &mut self,
        path: String,
        params: T,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let url = format!(
            "{}{}",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?,
            path
        );
        let res = self
            .http_client
            .patch(url.as_str())
            .headers(self.create_header(vec![])?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_put<T: Serialize>(
        &mut self,
        path: String,
        params: T,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let url = format!(
            "{}{}",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?,
            path
        );
        let res = self
            .http_client
            .put(url.as_str())
            .headers(self.create_header(vec![])?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn rest_delete(&mut self, path: String) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let url = format!(
            "{}{}",
            self.instance_url.as_ref().ok_or(Error::NotLoggedIn)?,
            path
        );
        let res = self
            .http_client
            .delete(url.as_str())
            .headers(self.create_header(vec![])?)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn get(
        &mut self,
        url: String,
        params: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let res = self
            .http_client
            .get(url.as_str())
            .headers(self.create_header(vec![])?)
            .query(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn get_raw(
        &mut self,
        url: &str,
        additional_headers: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let mut headers = self.create_header(additional_headers)?;
        headers.remove("Accept");
        let res = self.http_client.get(url).headers(headers).send().await?;
        Ok(res)
    }

    pub async fn post<T: Serialize>(
        &mut self,
        url: String,
        params: T,
        headers: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let res = self
            .http_client
            .post(url)
            .headers(self.create_header(headers)?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn post_raw_buffer(
        &mut self,
        url: String,
        body: Vec<u8>,
        headers: Vec<(String, String)>,
    ) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let res = self
            .http_client
            .post(url)
            .headers(self.create_header(headers)?)
            .body(body)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn put(&mut self, url: String, buffer: Vec<u8>) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let mut headers = self.create_header(vec![])?;
        headers.insert("Content-Type", HeaderValue::from_static("text/csv"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));
        let res = self
            .http_client
            .put(url.as_str())
            .headers(headers)
            .body(buffer)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn patch<T: Serialize>(&mut self, url: String, params: T) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let res = self
            .http_client
            .patch(url.as_str())
            .headers(self.create_header(vec![])?)
            .json(&params)
            .send()
            .await?;
        Ok(res)
    }

    pub async fn delete(&mut self, url: String) -> Result<Response, Error> {
        self.ensure_refresh().await?;

        let res = self
            .http_client
            .delete(url.as_str())
            .headers(self.create_header(vec![])?)
            .send()
            .await?;
        Ok(res)
    }

    fn create_header(&self, additional_headers: Vec<(String, String)>) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        let auth_value = format!(
            "Bearer {}",
            self.access_token.as_ref().ok_or(Error::NotLoggedIn)?.value
        );
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);

        //Default header
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        for (key, value) in additional_headers {
            let header_name: HeaderName = key
                .parse()
                .map_err(|_| Error::ConfigError(format!("Invalid header name: {}", key)))?;

            let header_value = HeaderValue::from_str(&value)?;

            //delete duplicates
            headers.remove(&header_name);

            headers.insert(header_name, header_value);
        }

        Ok(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    fn create_test_client(server_url: &str) -> Client {
        let mut client = Client::new();
        client.set_instance_url(server_url);
        client.set_access_token(
            "test_token".to_string(),
            // Use a timestamp far in the future so ensure_refresh doesn't trigger
            "9999999999000".to_string(),
            "Bearer".to_string(),
        );
        client
    }

    // --- Constructor and defaults ---

    #[test]
    fn test_new_defaults() {
        let client = Client::new();
        assert_eq!(client.login_endpoint, "https://login.salesforce.com");
        assert_eq!(client.version, "v60.0");
        assert!(client.client_id.is_none());
        assert!(client.client_secret.is_none());
        assert!(client.access_token.is_none());
        assert!(client.instance_url.is_none());
        assert!(client.refresh_token.is_none());
        assert!(client.secret_required);
    }

    #[test]
    fn test_default_calls_new() {
        let client = Client::default();
        assert_eq!(client.version, "v60.0");
    }

    // --- Setters and getters ---

    #[test]
    fn test_set_login_endpoint() {
        let mut client = Client::new();
        let result = client.set_login_endpoint("https://test.salesforce.com");
        assert_eq!(result.login_endpoint, "https://test.salesforce.com");
    }

    #[test]
    fn test_set_version() {
        let mut client = Client::new();
        client.set_version("v55.0");
        assert_eq!(client.version, "v55.0");
    }

    #[test]
    fn test_set_instance_url() {
        let mut client = Client::new();
        client.set_instance_url("https://example.com");
        assert_eq!(Some("https://example.com".to_string()), client.instance_url);
    }

    #[test]
    fn test_instance_url_getter() {
        let mut client = Client::new();
        assert_eq!(None, client.instance_url());
        client.set_instance_url("https://example.com");
        assert_eq!(Some("https://example.com"), client.instance_url());
    }

    #[test]
    fn test_set_refresh_token() {
        let mut client = Client::new();
        client.set_refresh_token("my_refresh_token");
        assert_eq!(Some("my_refresh_token".to_string()), client.refresh_token);
    }

    #[test]
    fn test_set_secret_required() {
        let mut client = Client::new();
        assert!(client.secret_required);
        client.set_secret_required(false);
        assert!(!client.secret_required);
    }

    #[test]
    fn test_set_client_id() {
        let mut client = Client::new();
        client.set_client_id("my_client_id");
        assert_eq!(Some("my_client_id".to_string()), client.client_id);
    }

    #[test]
    fn test_set_client_secret() {
        let mut client = Client::new();
        client.set_client_secret("my_secret");
        assert_eq!(Some("my_secret".to_string()), client.client_secret);
    }

    #[test]
    fn test_set_access_token() {
        let mut client = Client::new();
        client.set_access_token(
            "token_val".to_string(),
            "issued".to_string(),
            "Bearer".to_string(),
        );
        let token = client.access_token.as_ref().unwrap();
        assert_eq!("token_val", token.value);
        assert_eq!("issued", token.issued_at);
        assert_eq!("Bearer", token.token_type);
    }

    #[test]
    fn test_access_token_value() {
        let mut client = Client::new();
        assert_eq!(None, client.access_token_value());
        client.set_access_token("abc".to_string(), "".to_string(), "".to_string());
        assert_eq!(Some("abc"), client.access_token_value());
    }

    #[test]
    fn test_read_only_getters() {
        let mut client = Client::new();
        client.set_client_id("cid");
        client.set_client_secret("csecret");
        client.set_refresh_token("rtoken");

        assert_eq!(client.client_id(), Some("cid"));
        assert_eq!(client.client_secret(), Some("csecret"));
        assert_eq!(client.login_endpoint(), "https://login.salesforce.com");
        assert_eq!(client.version(), "v60.0");
        assert_eq!(client.refresh_token(), Some("rtoken"));
    }

    // --- Chaining setters ---

    #[test]
    fn test_setter_chaining() {
        let mut client = Client::new();
        client
            .set_login_endpoint("https://test.salesforce.com")
            .set_version("v55.0")
            .set_instance_url("https://inst.salesforce.com")
            .set_client_id("cid")
            .set_client_secret("csecret")
            .set_refresh_token("rtoken")
            .set_secret_required(false);

        assert_eq!(client.login_endpoint, "https://test.salesforce.com");
        assert_eq!(client.version, "v55.0");
        assert_eq!(
            client.instance_url,
            Some("https://inst.salesforce.com".to_string())
        );
        assert_eq!(client.client_id, Some("cid".to_string()));
        assert_eq!(client.client_secret, Some("csecret".to_string()));
        assert_eq!(client.refresh_token, Some("rtoken".to_string()));
        assert!(!client.secret_required);
    }

    // --- Refresh params ---

    #[test]
    fn test_get_refresh_params_with_secret() {
        let mut client = Client::new();
        client.set_client_id("cid");
        client.set_client_secret("csecret");
        client.set_refresh_token("rtoken");

        let params = client.get_refresh_params();
        assert_eq!(params.len(), 4);
        assert!(params.contains(&("grant_type".to_string(), "refresh_token".to_string())));
        assert!(params.contains(&("refresh_token".to_string(), "rtoken".to_string())));
        assert!(params.contains(&("client_id".to_string(), "cid".to_string())));
        assert!(params.contains(&("client_secret".to_string(), "csecret".to_string())));
    }

    #[test]
    fn test_get_refresh_params_without_secret() {
        let mut client = Client::new();
        client.set_secret_required(false);
        client.set_client_id("cid");
        client.set_refresh_token("rtoken");

        let params = client.get_refresh_params();
        assert_eq!(params.len(), 3);
        assert!(!params.iter().any(|(k, _)| k == "client_secret"));
    }

    #[test]
    fn test_get_refresh_params_defaults_when_none() {
        let client = Client::new();
        let params = client.get_refresh_params();
        // Should use empty strings for missing values
        assert!(params.contains(&("refresh_token".to_string(), "".to_string())));
        assert!(params.contains(&("client_id".to_string(), "".to_string())));
    }

    // --- create_header ---

    #[test]
    fn test_create_header_not_logged_in() {
        let client = Client::new();
        let result = client.create_header(vec![]);
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::NotLoggedIn => {}
            e => panic!("Expected NotLoggedIn, got {:?}", e),
        }
    }

    #[test]
    fn test_create_header_with_token() {
        let mut client = Client::new();
        client.set_access_token("mytoken".to_string(), "".to_string(), "Bearer".to_string());

        let headers = client.create_header(vec![]).unwrap();
        assert_eq!(
            headers.get("Authorization").unwrap(),
            "Bearer mytoken"
        );
        assert_eq!(headers.get("Accept").unwrap(), "application/json");
    }

    #[test]
    fn test_create_header_with_additional_headers() {
        let mut client = Client::new();
        client.set_access_token("mytoken".to_string(), "".to_string(), "Bearer".to_string());

        let headers = client
            .create_header(vec![
                ("X-Custom".to_string(), "custom_value".to_string()),
                ("Accept".to_string(), "text/xml".to_string()),
            ])
            .unwrap();

        assert_eq!(headers.get("X-Custom").unwrap(), "custom_value");
        // Accept should be overridden
        assert_eq!(headers.get("Accept").unwrap(), "text/xml");
    }

    // --- ensure_refresh ---

    #[tokio::test]
    async fn test_ensure_refresh_no_token() {
        let mut client = Client::new();
        // Should return Ok without doing anything
        let result = client.ensure_refresh().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ensure_refresh_token_not_expired() {
        let mut client = Client::new();
        // Set a token with issued_at far in the future
        client.set_access_token(
            "token".to_string(),
            "9999999999000".to_string(),
            "Bearer".to_string(),
        );
        let result = client.ensure_refresh().await;
        assert!(result.is_ok());
        assert_eq!(client.access_token_value(), Some("token"));
    }

    // --- login_with_credential ---

    #[tokio::test]
    async fn test_login_with_credential_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/oauth2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "PowerLevel9000",
                    "issued_at": "1234567890000",
                    "id": "https://login.salesforce.com/id/00Dxx/005xx",
                    "instance_url": server.url(),
                    "signature": "sig",
                    "token_type": "Bearer",
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut client = Client::new();
        client.set_client_id("cid");
        client.set_client_secret("csecret");
        client.set_login_endpoint(&server.url());

        let result = client
            .login_with_credential("user", "pass")
            .await;

        assert!(result.is_ok());
        assert_eq!(client.access_token_value(), Some("PowerLevel9000"));
        assert_eq!(client.instance_url.unwrap(), server.url());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_login_with_credential_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/oauth2/token")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "error": "invalid_grant",
                    "error_description": "authentication failure"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut client = Client::new();
        client.set_client_id("cid");
        client.set_client_secret("csecret");
        client.set_login_endpoint(&server.url());

        let result = client
            .login_with_credential("user", "pass")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::TokenError(_) => {}
            e => panic!("Expected TokenError, got {:?}", e),
        }
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_login_with_credential_missing_client_id() {
        let mut client = Client::new();
        let result = client.login_with_credential("user", "pass").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::ConfigError(msg) => assert!(msg.contains("client_id")),
            e => panic!("Expected ConfigError, got {:?}", e),
        }
    }

    // --- refresh ---

    #[tokio::test]
    async fn test_refresh_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/oauth2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "new_token",
                    "issued_at": "1234567890000",
                    "id": "https://login.salesforce.com/id/00Dxx/005xx",
                    "instance_url": server.url(),
                    "signature": "sig",
                    "token_type": "Bearer",
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut client = Client::new();
        client.set_client_id("cid");
        client.set_client_secret("csecret");
        client.set_refresh_token("rtoken");
        client.set_login_endpoint(&server.url());

        let result = client.refresh().await;
        assert!(result.is_ok());
        assert_eq!(client.access_token_value(), Some("new_token"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_refresh_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/oauth2/token")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "error": "invalid_grant",
                    "error_description": "expired refresh token"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut client = Client::new();
        client.set_login_endpoint(&server.url());

        let result = client.refresh().await;
        assert!(result.is_err());
        mock.assert_async().await;
    }

    // --- login_by_soap ---

    #[tokio::test]
    async fn test_login_by_soap_success() {
        let mut server = Server::new_async().await;
        let soap_response = r#"<?xml version="1.0" encoding="UTF-8"?>
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
                <soapenv:Body>
                    <loginResponse>
                        <result>
                            <sessionId>soap_token_123</sessionId>
                            <serverUrl>https://na1.salesforce.com/services/Soap/u/60.0/00Dxx</serverUrl>
                            <serverTimestamp>2024-01-01T00:00:00.000Z</serverTimestamp>
                        </result>
                    </loginResponse>
                </soapenv:Body>
            </soapenv:Envelope>"#;

        let mock = server
            .mock("POST", "/services/Soap/u/v60.0")
            .with_status(200)
            .with_header("content-type", "text/xml")
            .with_body(soap_response)
            .create_async()
            .await;

        let mut client = Client::new();
        client.set_login_endpoint(&server.url());

        let result = client
            .login_by_soap("user", "pass")
            .await;

        assert!(result.is_ok());
        let token = client.access_token.unwrap();
        assert_eq!(token.value, "soap_token_123");
        assert_eq!(token.token_type, "Bearer");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_login_by_soap_failure() {
        let mut server = Server::new_async().await;
        let soap_error = r#"<?xml version="1.0" encoding="UTF-8"?>
            <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
                <soapenv:Body>
                    <soapenv:Fault>
                        <faultcode>INVALID_LOGIN</faultcode>
                        <faultstring>Invalid username or password</faultstring>
                    </soapenv:Fault>
                </soapenv:Body>
            </soapenv:Envelope>"#;

        let mock = server
            .mock("POST", "/services/Soap/u/v60.0")
            .with_status(500)
            .with_header("content-type", "text/xml")
            .with_body(soap_error)
            .create_async()
            .await;

        let mut client = Client::new();
        client.set_login_endpoint(&server.url());

        let result = client
            .login_by_soap("user", "pass")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::LoginError(e) => {
                assert_eq!(e.error_code, "INVALID_LOGIN");
                assert_eq!(e.message, "Invalid username or password");
            }
            e => panic!("Expected LoginError, got {:?}", e),
        }
        mock.assert_async().await;
    }

    // --- HTTP methods with mock server ---

    #[tokio::test]
    async fn test_get() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .match_query(mockito::Matcher::AllOf(vec![]))
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client.get(format!("{}/test", server.url()), vec![]).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().status(), 200);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(201)
            .with_body(r#"{"id":"123","success":true}"#)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let mut params = std::collections::HashMap::new();
        params.insert("Name", "Test");
        let res = client
            .post(format!("{}/test", server.url()), params, vec![])
            .await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().status(), 201);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_patch() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PATCH", "/test")
            .with_status(204)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let mut params = std::collections::HashMap::new();
        params.insert("Name", "Updated");
        let res = client
            .patch(format!("{}/test", server.url()), params)
            .await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().status(), 204);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_delete() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/test")
            .with_status(204)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client.delete(format!("{}/test", server.url())).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().status(), 204);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_put() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/test")
            .with_status(201)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client
            .put(format!("{}/test", server.url()), b"csv,data".to_vec())
            .await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().status(), 201);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_raw_buffer() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/test")
            .with_status(200)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client
            .post_raw_buffer(
                format!("{}/test", server.url()),
                b"raw data".to_vec(),
                vec![("Content-Type".to_string(), "text/csv".to_string())],
            )
            .await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_raw() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_body("raw response")
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client
            .get_raw(
                &format!("{}/test", server.url()),
                vec![],
            )
            .await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    // --- REST methods ---

    #[tokio::test]
    async fn test_rest_get() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/some/path")
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client.rest_get("/some/path".to_string(), vec![]).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rest_post() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/some/path")
            .with_status(201)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let mut params = std::collections::HashMap::new();
        params.insert("key", "value");
        let res = client.rest_post("/some/path".to_string(), params).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rest_patch() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PATCH", "/some/path")
            .with_status(204)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let mut params = std::collections::HashMap::new();
        params.insert("key", "value");
        let res = client.rest_patch("/some/path".to_string(), params).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rest_put() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/some/path")
            .with_status(200)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let mut params = std::collections::HashMap::new();
        params.insert("key", "value");
        let res = client.rest_put("/some/path".to_string(), params).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_rest_delete() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("DELETE", "/some/path")
            .with_status(204)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client.rest_delete("/some/path".to_string()).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    // --- get_identity ---

    #[tokio::test]
    async fn test_get_identity_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/id/info")
            .with_status(200)
            .with_body(r#"{"user_id":"005xx","username":"test@test.com"}"#)
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let result = client
            .get_identity(format!("{}/id/info", server.url()))
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test@test.com"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_identity_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/id/info")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "message": "Session expired",
                    "errorCode": "INVALID_SESSION_ID"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let result = client
            .get_identity(format!("{}/id/info", server.url()))
            .await;
        assert!(result.is_err());
        mock.assert_async().await;
    }

    // --- rest_get_fulluri ---

    #[tokio::test]
    async fn test_rest_get_fulluri() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/apexrest/MyEndpoint")
            .match_query(mockito::Matcher::UrlEncoded("param".into(), "value".into()))
            .with_status(200)
            .with_body("ok")
            .create_async()
            .await;

        let mut client = create_test_client(&server.url());
        let res = client
            .rest_get_fulluri("MyEndpoint?param=value")
            .await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    // --- ensure_refresh with expired token ---

    #[tokio::test]
    async fn test_ensure_refresh_expired_token_triggers_refresh() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/oauth2/token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": "refreshed_token",
                    "issued_at": "9999999999000",
                    "id": "https://login.salesforce.com/id/00Dxx/005xx",
                    "instance_url": server.url(),
                    "signature": "sig",
                    "token_type": "Bearer",
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut client = Client::new();
        client.set_login_endpoint(&server.url());
        client.set_client_id("cid");
        client.set_client_secret("csecret");
        client.set_refresh_token("rtoken");
        // Set a token with issued_at far in the past (expired > 2 hours ago)
        client.set_access_token(
            "old_token".to_string(),
            "1000000000000".to_string(), // ~2001, well past 2 hours
            "Bearer".to_string(),
        );

        let result = client.ensure_refresh().await;
        assert!(result.is_ok());
        assert_eq!(client.access_token_value(), Some("refreshed_token"));
        mock.assert_async().await;
    }
}
