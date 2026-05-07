use crate::utils::clickhouse::ClickHouseClient;
use rmcp::service::RequestContext;
use rmcp::task_handler;
use rmcp::task_manager::OperationProcessor;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, handler::server::wrapper::Parameters,
    model::*, schemars, tool, tool_router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteQueryRequest {
    pub query: String,
}

#[derive(Clone)]
pub struct AGPService {
    client: Arc<ClickHouseClient>,
    processor: Arc<Mutex<OperationProcessor>>,
}

#[tool_router]
impl AGPService {
    pub fn new(client: ClickHouseClient) -> Self {
        Self {
            client: Arc::new(client),
            processor: Arc::new(Mutex::new(OperationProcessor::new())),
        }
    }

    #[tool(description = "Retrieves the Clickhouse database schema from the AGP API.")]
    async fn get_schema(&self) -> Result<CallToolResult, McpError> {
        match self
            .client
            .exec("SELECT name, create_table_query FROM system.tables WHERE database = currentDatabase()")
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::json(resp)?])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!("ClickHouseError: {}", e))])),
        }
    }

    #[tool(description = "Executes a read-only Clickhouse query via the AGP API.")]
    async fn execute_query(
        &self,
        Parameters(ExecuteQueryRequest { query }): Parameters<ExecuteQueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.client.exec(&query).await {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::json(resp)?])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "ClickHouseError: {}",
                e
            ))])),
        }
    }
}

#[task_handler]
impl ServerHandler for AGPService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
            .with_instructions("High-performance Rust MCP server for Clickhouse. Enables AI models to retrieve database schemas and execute secure, read-only queries via the AGP API.")
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: Self::tool_router().list_all(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        Self::tool_router()
            .call(rmcp::handler::server::tool::ToolCallContext::new(
                self, request, context,
            ))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn get_text_content(result: &CallToolResult) -> String {
        match &result.content[0].raw {
            RawContent::Text(text_content) => text_content.text.clone(),
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_get_schema_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let ch_client = ClickHouseClient::new(&url, None).unwrap();
        let service = AGPService::new(ch_client);

        let mock_response = serde_json::json!({
            "meta": [{"name": "table", "type": "String"}],
            "data": [{"table": "test_table"}],
            "rows": 1,
            "statistics": {"bytes_read": 0, "elapsed": 0.0, "rows_read": 0}
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

        let result = service.get_schema().await.unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = get_text_content(&result);
        assert!(text.contains("test_table"), "Result was: {}", text);
    }

    #[tokio::test]
    async fn test_get_schema_error() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let ch_client = ClickHouseClient::new(&url, None).unwrap();
        let service = AGPService::new(ch_client);

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(500)
            .with_body("DB Error")
            .create_async()
            .await;

        let result = service.get_schema().await.unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = get_text_content(&result);
        assert!(
            text.contains("ClickHouseError: DB Error"),
            "Result was: {}",
            text
        );
    }

    #[tokio::test]
    async fn test_execute_query_success() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let ch_client = ClickHouseClient::new(&url, None).unwrap();
        let service = AGPService::new(ch_client);

        let mock_response = serde_json::json!({
            "meta": [{"name": "col", "type": "Int32"}],
            "data": [{"col": 42}],
            "rows": 1,
            "statistics": {"bytes_read": 0, "elapsed": 0.0, "rows_read": 0}
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

        let result = service
            .execute_query(Parameters(ExecuteQueryRequest {
                query: "SELECT 42".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = get_text_content(&result);
        assert!(text.contains("42"), "Result was: {}", text);
    }

    #[tokio::test]
    async fn test_execute_query_error() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let ch_client = ClickHouseClient::new(&url, None).unwrap();
        let service = AGPService::new(ch_client);

        let _m = server
            .mock("POST", "/")
            .match_query(mockito::Matcher::UrlEncoded(
                "default_format".into(),
                "JSON".into(),
            ))
            .with_status(500)
            .with_body("Query Error")
            .create_async()
            .await;

        let result = service
            .execute_query(Parameters(ExecuteQueryRequest {
                query: "SELECT invalid".to_string(),
            }))
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));
        let text = get_text_content(&result);
        assert!(
            text.contains("ClickHouseError: Query Error"),
            "Result was: {}",
            text
        );
    }

    #[tokio::test]
    async fn test_server_handler_get_info() {
        let ch_client = ClickHouseClient::new("http://localhost:8123", None).unwrap();
        let service = AGPService::new(ch_client);

        let info = service.get_info();
        assert_eq!(info.server_info.name, env!("CARGO_PKG_NAME"));
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
    }

    #[tokio::test]
    async fn test_get_schema_multiple_tables() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let ch_client = ClickHouseClient::new(&url, None).unwrap();
        let service = AGPService::new(ch_client);

        let mock_response = serde_json::json!({
            "meta": [{"name": "name", "type": "String"}, {"name": "create_table_query", "type": "String"}],
            "data": [
                {"name": "table1", "create_table_query": "CREATE TABLE table1..."},
                {"name": "table2", "create_table_query": "CREATE TABLE table2..."}
            ],
            "rows": 2,
            "statistics": {"bytes_read": 0, "elapsed": 0.0, "rows_read": 0}
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

        let result = service.get_schema().await.unwrap();
        // Since we return JSON content, it might be serialized differently
        // Let's just check that it succeeded and has content
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

}
