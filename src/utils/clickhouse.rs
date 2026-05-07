//! ClickHouse client and response types.
//!
//! This module provides a high-level client for interacting with a ClickHouse proxy API,
//! handling JSON responses, and providing structured error information.

use anyhow::Result;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when interacting with ClickHouse.
#[derive(Debug, Error)]
pub enum ClickHouseError {
    /// An error returned by the ClickHouse server or proxy.
    #[error("ClickHouse error: {0}")]
    Server(String),

    /// An error that occurred while sending the request or receiving the response.
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// An error that occurred while parsing the JSON response.
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    /// An error that occurred while parsing the URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

/// A column descriptor in a ClickHouse JSON response.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ColumnDescriptor {
    /// The name of the column.
    pub name: String,
    /// The ClickHouse type of the column.
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Statistics about a ClickHouse query execution.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Statistics {
    /// Total bytes read during the query.
    pub bytes_read: u64,
    /// Total time elapsed during the query in seconds.
    pub elapsed: f64,
    /// Total rows read during the query.
    pub rows_read: u64,
}

/// The standard ClickHouse JSON response format.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ClickHouseResponse {
    /// Metadata about the result columns.
    pub meta: Vec<ColumnDescriptor>,
    /// The actual query result data as a list of JSON objects.
    pub data: Vec<serde_json::Value>,
    /// Total number of rows in the result.
    pub rows: u64,
    /// Execution statistics.
    pub statistics: Statistics,
}

/// A client for interacting with ClickHouse via a proxy API.
#[derive(Debug, Clone)]
pub struct ClickHouseClient {
    /// The URL of the ClickHouse proxy.
    proxy_url: Url,
    /// The underlying HTTP client.
    client: Client,
}

impl ClickHouseClient {
    /// Creates a new `ClickHouseClient`.
    ///
    /// # Arguments
    ///
    /// * `proxy_url` - The URL of the ClickHouse proxy.
    /// * `initial_headers` - Optional headers to include in every request (e.g., authentication).
    pub fn new(proxy_url: &str, initial_headers: Option<HeaderMap>) -> Result<Self, ClickHouseError> {
        let mut url = Url::parse(proxy_url)?;
        url.query_pairs_mut().append_pair("default_format", "JSON");

        let mut headers = initial_headers.unwrap_or_default();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(ClickHouseError::Request)?;

        Ok(Self {
            proxy_url: url,
            client,
        })
    }

    /// Executes a read-only SQL query.
    ///
    /// # Arguments
    ///
    /// * `query` - The SQL query to execute.
    pub async fn exec(&self, query: &str) -> Result<ClickHouseResponse, ClickHouseError> {
        let response = self
            .client
            .post(self.proxy_url.as_str())
            .body(query.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClickHouseError::Server(err_text));
        }

        let body_bytes = response.bytes().await?;

        // In ClickHouse, the response can contain an 'exception' field even if it's JSON.
        // We first try to parse it as a regular response, then check for exception.
        let json_value: serde_json::Value = serde_json::from_slice(&body_bytes)?;

        if let Some(exception) = json_value.get("exception")
            && let Some(msg) = exception.as_str()
        {
            return Err(ClickHouseError::Server(msg.to_string()));
        }

        let result: ClickHouseResponse = serde_json::from_value(json_value)?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn test_new_invalid_url() {
        let result = ClickHouseClient::new("not a url", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_success() {
        let client = ClickHouseClient::new("http://localhost:8123", None).unwrap();
        assert_eq!(
            client.proxy_url.as_str(),
            "http://localhost:8123/?default_format=JSON"
        );
    }

    #[test]
    fn test_new_with_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Test", HeaderValue::from_static("test-val"));
        let _client = ClickHouseClient::new("http://localhost:8123", Some(headers)).unwrap();
    }

    #[tokio::test]
    async fn test_exec_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let client = ClickHouseClient::new(&url, None).unwrap();

        let mock_response = serde_json::json!({
            "meta": [{"name": "col1", "type": "String"}],
            "data": [{"col1": "val1"}],
            "rows": 1,
            "statistics": {
                "bytes_read": 10,
                "elapsed": 0.1,
                "rows_read": 1
            }
        });

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&mock_response).unwrap())
            .create_async()
            .await;

        let result = client.exec("SELECT 1").await;
        match result {
            Ok(res) => {
                assert_eq!(res.rows, 1);
                assert_eq!(res.meta[0].name, "col1");
            }
            Err(e) => panic!("Expected success, got error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_exec_http_error() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let client = ClickHouseClient::new(&url, None).unwrap();

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let result = client.exec("SELECT 1").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Internal Server Error"),
            "Error message was: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_exec_clickhouse_exception() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let client = ClickHouseClient::new(&url, None).unwrap();

        let mock_response = serde_json::json!({
            "exception": "Some ClickHouse error"
        });

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(serde_json::to_string(&mock_response).unwrap())
            .create_async()
            .await;

        let result = client.exec("SELECT 1").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Some ClickHouse error"),
            "Error message was: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_exec_invalid_json() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let client = ClickHouseClient::new(&url, None).unwrap();

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(200)
            .with_body("not json")
            .create_async()
            .await;

        let result = client.exec("SELECT 1").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse response"),
            "Error message was: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_exec_empty_data() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let client = ClickHouseClient::new(&url, None).unwrap();

        let mock_response = serde_json::json!({
            "meta": [{"name": "col1", "type": "String"}],
            "data": [],
            "rows": 0,
            "statistics": {
                "bytes_read": 0,
                "elapsed": 0.0,
                "rows_read": 0
            }
        });

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(200)
            .with_body(serde_json::to_string(&mock_response).unwrap())
            .create_async()
            .await;

        let result = client.exec("SELECT 1 WHERE 0").await.unwrap();
        assert_eq!(result.rows, 0);
        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn test_exec_missing_statistics() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let client = ClickHouseClient::new(&url, None).unwrap();

        let mock_response = serde_json::json!({
            "meta": [],
            "data": [],
            "rows": 0
        });

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(200)
            .with_body(serde_json::to_string(&mock_response).unwrap())
            .create_async()
            .await;

        let result = client.exec("SELECT 1").await;
        assert!(result.is_err());
    }
}
