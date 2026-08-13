use crate::client::client::Client;
use crate::errors::Error;
use reqwest::Response;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Default)]
pub struct BulkApi {
    pub(crate) client: Client,
}

impl BulkApi {
    pub fn new(client: Client) -> Self {
        BulkApi { client }
    }

    fn base_path(&self) -> Result<String, Error> {
        let instance_url = self
            .client
            .instance_url
            .as_ref()
            .ok_or(Error::NotLoggedIn)?;

        let version = &self.client.version[1..];
        Ok(format!("{}/services/async/{}", instance_url, version))
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_jobs_create.htm>
    pub async fn create_job<T: Serialize>(&mut self, params: T) -> Result<Response, Error> {
        let resource_url = format!("{}/job", self.base_path()?);
        let headers = self.get_auth_headers()?;
        self.client.post(resource_url, params, headers).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_quickstart_add_batch.htm>
    pub async fn add_batch_job(&mut self, job_id: &str, csv: Vec<u8>) -> Result<Response, Error> {
        let resource_url = format!("{}/job/{}/batch", self.base_path()?, job_id);
        let mut headers = self.get_auth_headers()?;
        headers.push(("Content-Type".to_string(), "text/csv".to_string()));
        self.client
            .post_raw_buffer(resource_url, csv, headers)
            .await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_quickstart_check_status.htm>
    pub async fn get_batch(&mut self, job_id: &str, batch_id: &str) -> Result<Response, Error> {
        let resource_url = format!("{}/job/{}/batch/{}/", self.base_path()?, job_id, batch_id);
        let headers = self.get_auth_headers()?;
        self.client.get_raw(&resource_url, headers).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_jobs_close.htm>
    pub async fn close_job(&mut self, job_id: &str) -> Result<Response, Error> {
        let resource_url = format!("{}/job/{}", self.base_path()?, job_id);
        let headers = self.get_auth_headers()?;
        let mut params = HashMap::new();
        params.insert("state", "Closed");
        self.client.post(resource_url, params, headers).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_jobs_get_details.htm>
    pub async fn get_job_details(&mut self, job_id: &str) -> Result<Response, Error> {
        let resource_url = format!("{}/job/{}", self.base_path()?, job_id);
        let headers = self.get_auth_headers()?;
        self.client.get_raw(&resource_url, headers).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_batches_get_info_all.htm>
    pub async fn get_batches(
        &mut self,
        job_id: &str,
        content_type: &str,
    ) -> Result<Response, Error> {
        let resource_url = format!("{}/job/{}/batch", self.base_path()?, job_id);
        let mut headers = self.get_auth_headers()?;
        headers.push(("Content-Type".to_string(), content_type.to_string()));
        self.client.get_raw(&resource_url, headers).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_batches_get_results.htm>
    pub async fn get_result_list(
        &mut self,
        job_id: &str,
        batch_id: &str,
        content_type: &str,
    ) -> Result<Response, Error> {
        let resource_url = format!(
            "{}/job/{}/batch/{}/result",
            self.base_path()?,
            job_id,
            batch_id
        );
        let mut headers = self.get_auth_headers()?;
        headers.push(("Content-Type".to_string(), content_type.to_string()));
        self.client.get_raw(&resource_url, headers).await
    }

    /// Retrieve bulk query results.
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_code_curl_walkthrough_pk_chunking.htm>
    pub async fn get_result(
        &mut self,
        job_id: &str,
        batch_id: &str,
        result_id: &str,
    ) -> Result<Response, Error> {
        let resource_url = format!(
            "{}/job/{}/batch/{}/result/{}",
            self.base_path()?,
            job_id,
            batch_id,
            result_id
        );
        let mut headers = self.get_auth_headers()?;
        headers.push(("Content-Type".to_string(), "application/xml".to_string()));
        self.client.get_raw(&resource_url, headers).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/asynch_api_jobs_abort.htm>
    pub async fn abort_job(&mut self, job_id: &str) -> Result<Response, Error> {
        let resource_url = format!("{}/job/{}", self.base_path()?, job_id);
        let mut headers = self.get_auth_headers()?;
        headers.push(("Content-Type".to_string(), "application/json".to_string()));

        let mut params = HashMap::new();
        params.insert("state", "Aborted");

        self.client.post(resource_url, params, headers).await
    }

    fn get_auth_headers(&self) -> Result<Vec<(String, String)>, Error> {
        let token = self
            .client
            .access_token
            .as_ref()
            .ok_or(Error::NotLoggedIn)?;
        Ok(vec![
            //X-SFDC-Session is needed for API v1 we can just pass it our access token
            (
                "X-SFDC-Session".to_string(),
                token.value.clone(),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    fn create_test_bulk_api(server_url: &str) -> BulkApi {
        let mut client = Client::new();
        client.set_instance_url(server_url);
        client.set_access_token(
            "test_token".to_string(),
            "9999999999000".to_string(),
            "Bearer".to_string(),
        );
        client.set_version("v60.0");
        BulkApi::new(client)
    }

    #[test]
    fn test_new() {
        let client = Client::new();
        let api = BulkApi::new(client);
        assert!(api.client.instance_url.is_none());
    }

    #[test]
    fn test_base_path() {
        let mut client = Client::new();
        client.set_instance_url("https://na1.salesforce.com");
        client.set_version("v60.0");
        let api = BulkApi::new(client);
        // v60.0 -> strips the 'v' to get 60.0
        assert_eq!(
            api.base_path().unwrap(),
            "https://na1.salesforce.com/services/async/60.0"
        );
    }

    #[test]
    fn test_base_path_not_logged_in() {
        let client = Client::new();
        let api = BulkApi::new(client);
        let result = api.base_path();
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::NotLoggedIn => {}
            e => panic!("Expected NotLoggedIn, got {:?}", e),
        }
    }

    #[test]
    fn test_get_auth_headers() {
        let mut client = Client::new();
        client.set_access_token(
            "my_session_token".to_string(),
            "".to_string(),
            "Bearer".to_string(),
        );
        let api = BulkApi::new(client);
        let headers = api.get_auth_headers().unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "X-SFDC-Session");
        assert_eq!(headers[0].1, "my_session_token");
    }

    #[test]
    fn test_get_auth_headers_not_logged_in() {
        let client = Client::new();
        let api = BulkApi::new(client);
        let result = api.get_auth_headers();
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::NotLoggedIn => {}
            e => panic!("Expected NotLoggedIn, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_job() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/async/60.0/job")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(json!({"id": "750xx000000001"}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let mut params = HashMap::new();
        params.insert("operation", "insert");
        params.insert("object", "Account");
        let res = api.create_job(params).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_add_batch_job() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/async/60.0/job/750xx/batch")
            .with_status(201)
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let csv = b"Name\nTest Account".to_vec();
        let res = api.add_batch_job("750xx", csv).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_batch() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/async/60.0/job/750xx/batch/751xx/")
            .with_status(200)
            .with_body("batch info")
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let res = api.get_batch("750xx", "751xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_close_job() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/async/60.0/job/750xx")
            .with_status(200)
            .with_body(json!({"state": "Closed"}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let res = api.close_job("750xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_job_details() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/async/60.0/job/750xx")
            .with_status(200)
            .with_body("job details")
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let res = api.get_job_details("750xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_batches() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/async/60.0/job/750xx/batch")
            .with_status(200)
            .with_body("batches")
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let res = api.get_batches("750xx", "application/json").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_result_list() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/async/60.0/job/750xx/batch/751xx/result")
            .with_status(200)
            .with_body("results")
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let res = api
            .get_result_list("750xx", "751xx", "application/json")
            .await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_result() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/services/async/60.0/job/750xx/batch/751xx/result/752xx",
            )
            .with_status(200)
            .with_body("result data")
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let res = api.get_result("750xx", "751xx", "752xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_abort_job() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/services/async/60.0/job/750xx")
            .with_status(200)
            .with_body(json!({"state": "Aborted"}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api(&server.url());
        let res = api.abort_job("750xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }
}
