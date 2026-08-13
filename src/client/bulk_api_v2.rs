use crate::client::client::Client;
use crate::errors::Error;
use reqwest::Response;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Default)]
pub struct BulkApiV2 {
    pub(crate) client: Client,
}

impl BulkApiV2 {
    pub fn new(client: Client) -> Self {
        BulkApiV2 { client }
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/create_job.htm>
    pub async fn create_job<T: Serialize>(&mut self, params: T) -> Result<Response, Error> {
        let resource_url = format!("{}/jobs/ingest", self.client.base_path()?);
        self.client.post(resource_url, params, vec![]).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/upload_job_data.htm>
    pub async fn upload_job_data(&mut self, job_id: &str, csv: Vec<u8>) -> Result<String, Error> {
        let resource_url = format!("{}/jobs/ingest/{}/batches", self.client.base_path()?, job_id);
        let res = self.client.put(resource_url, csv).await?;

        if res.status().is_success() {
            Ok("Created".to_string())
        } else {
            Err(Error::DescribeError(res.json().await?))
        }
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/get_all_jobs.htm>
    pub async fn get_all_jobs(&mut self) -> Result<Response, Error> {
        let resource_url = format!("{}/jobs/ingest/", self.client.base_path()?);
        self.client.get(resource_url, vec![]).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/get_job_info.htm>
    pub async fn get_job_info(&mut self, job_id: &str) -> Result<Response, Error> {
        let resource_url = format!("{}/jobs/ingest/{}", self.client.base_path()?, job_id);
        self.client.get(resource_url, vec![]).await
    }

    /// Get job records (successful, failed, or unprocessed).
    ///
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/get_job_successful_results.htm>
    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/get_job_failed_results.htm>
    pub async fn get_job_records(
        &mut self,
        job_id: &str,
        result_set: &str,
    ) -> Result<Response, Error> {
        // NOTE: RESULT_SET IS ONE OF successfulResults, failedResults, unprocessedrecords
        let resource_url = format!(
            "{}/jobs/ingest/{}/{}",
            self.client.base_path()?,
            job_id,
            result_set
        );
        self.client.get_raw(&resource_url, vec![]).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/abort_job.htm>
    pub async fn abort_job(&mut self, job_id: &str) -> Result<Response, Error> {
        let resource_url = format!("{}/jobs/ingest/{}", self.client.base_path()?, job_id);
        let mut params = HashMap::new();
        params.insert("state", "Aborted");
        self.client.patch(resource_url, params).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/close_job.htm>
    pub async fn set_upload_state<T: Serialize>(
        &mut self,
        job_id: &str,
        params: T,
    ) -> Result<Response, Error> {
        let resource_url = format!("{}/jobs/ingest/{}", self.client.base_path()?, job_id);
        self.client.patch(resource_url, params).await
    }

    /// <https://developer.salesforce.com/docs/atlas.en-us.api_asynch.meta/api_asynch/get_job_info.htm>
    pub async fn check_job_status(&mut self, job_id: &str) -> Result<Response, Error> {
        let resource_url = format!("{}/jobs/ingest/{}/", self.client.base_path()?, job_id);
        self.client.get(resource_url, vec![]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;

    fn create_test_bulk_api_v2(server_url: &str) -> BulkApiV2 {
        let mut client = Client::new();
        client.set_instance_url(server_url);
        client.set_access_token(
            "test_token".to_string(),
            "9999999999000".to_string(),
            "Bearer".to_string(),
        );
        client.set_version("v60.0");
        BulkApiV2::new(client)
    }

    #[test]
    fn test_new() {
        let client = Client::new();
        let api = BulkApiV2::new(client);
        assert!(api.client.instance_url.is_none());
    }

    #[test]
    fn test_base_path() {
        let mut client = Client::new();
        client.set_instance_url("https://na1.salesforce.com");
        client.set_version("v60.0");
        let api = BulkApiV2::new(client);
        assert_eq!(
            api.client.base_path().unwrap(),
            "https://na1.salesforce.com/services/data/v60.0"
        );
    }

    #[test]
    fn test_base_path_not_logged_in() {
        let client = Client::new();
        let api = BulkApiV2::new(client);
        let result = api.client.base_path();
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
            .mock("POST", "/services/data/v60.0/jobs/ingest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": "750xx",
                    "operation": "insert",
                    "object": "Account",
                    "createdById": "005xx",
                    "createdDate": "2024-01-01",
                    "systemModstamp": "2024-01-01",
                    "state": "Open",
                    "concurrencyMode": "Parallel",
                    "contentType": "CSV",
                    "apiVersion": 60.0,
                    "contentUrl": "services/data/v60.0/jobs/ingest/750xx/batches",
                    "lineEnding": "LF"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let mut params = HashMap::new();
        params.insert("operation", "insert");
        params.insert("object", "Account");
        let res = api.create_job(params).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_upload_job_data_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/services/data/v60.0/jobs/ingest/750xx/batches")
            .with_status(201)
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let csv = b"Name\nTest Account".to_vec();
        let res = api.upload_job_data("750xx", csv).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "Created");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_upload_job_data_failure() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PUT", "/services/data/v60.0/jobs/ingest/750xx/batches")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "message": "Invalid CSV",
                    "errorCode": "INVALID_CONTENT"
                })
                .to_string(),
            )
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let csv = b"bad data".to_vec();
        let res = api.upload_job_data("750xx", csv).await;
        assert!(res.is_err());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_all_jobs() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/data/v60.0/jobs/ingest/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"records": [], "done": true}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let res = api.get_all_jobs().await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_job_info() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/data/v60.0/jobs/ingest/750xx")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"id": "750xx", "state": "Open"}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let res = api.get_job_info("750xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_get_job_records() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/services/data/v60.0/jobs/ingest/750xx/successfulResults",
            )
            .with_status(200)
            .with_body("sf__Id,Name\n001xx,Test")
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let res = api.get_job_records("750xx", "successfulResults").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_abort_job() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PATCH", "/services/data/v60.0/jobs/ingest/750xx")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"state": "Aborted"}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let res = api.abort_job("750xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_set_upload_state() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("PATCH", "/services/data/v60.0/jobs/ingest/750xx")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"state": "UploadComplete"}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let mut params = HashMap::new();
        params.insert("state", "UploadComplete");
        let res = api.set_upload_state("750xx", params).await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_check_job_status() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/services/data/v60.0/jobs/ingest/750xx/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({"id": "750xx", "state": "JobComplete"}).to_string())
            .create_async()
            .await;

        let mut api = create_test_bulk_api_v2(&server.url());
        let res = api.check_job_status("750xx").await;
        assert!(res.is_ok());
        mock.assert_async().await;
    }
}
