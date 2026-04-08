use std::{collections::BTreeMap, sync::Arc};

use base64::Engine;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use crate::{
    http_client::{FetchOutcome, HttpClient},
    types::{
        BinaryResponse, DownloadBinaryRequest, FetchRequest, FetchResponse, GetJsonRequest,
        JsonResponse, PostJsonRequest,
    },
};

#[derive(Clone)]
pub struct NetworkProxy {
    http: Arc<HttpClient>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NetworkProxy {
    pub fn new(http: Arc<HttpClient>) -> Self {
        Self {
            http,
            tool_router: Self::tool_router(),
        }
    }

    /// Internal helper used by both the `fetch` tool and integration tests.
    pub async fn fetch_inner(&self, req: FetchRequest) -> anyhow::Result<FetchResponse> {
        let outcome = self.http.execute(req).await?;
        Ok(outcome_to_fetch_response(outcome))
    }

    pub async fn get_json_inner(&self, req: GetJsonRequest) -> anyhow::Result<JsonResponse> {
        let outcome = self
            .http
            .execute(FetchRequest {
                method: "GET".to_string(),
                url: req.url,
                headers: with_default_accept_json(req.headers),
                body: None,
                body_base64: None,
                timeout_ms: req.timeout_ms,
                follow_redirects: true,
                max_response_bytes: 10 * 1024 * 1024,
            })
            .await?;
        outcome_to_json_response(outcome)
    }

    pub async fn post_json_inner(&self, req: PostJsonRequest) -> anyhow::Result<JsonResponse> {
        let mut headers = req.headers;
        headers
            .entry("content-type".to_string())
            .or_insert_with(|| "application/json".to_string());
        headers
            .entry("accept".to_string())
            .or_insert_with(|| "application/json".to_string());
        let body = serde_json::to_string(&req.json)?;
        let outcome = self
            .http
            .execute(FetchRequest {
                method: "POST".to_string(),
                url: req.url,
                headers,
                body: Some(body),
                body_base64: None,
                timeout_ms: req.timeout_ms,
                follow_redirects: true,
                max_response_bytes: 10 * 1024 * 1024,
            })
            .await?;
        outcome_to_json_response(outcome)
    }

    pub async fn download_binary_inner(
        &self,
        req: DownloadBinaryRequest,
    ) -> anyhow::Result<BinaryResponse> {
        let outcome = self
            .http
            .execute(FetchRequest {
                method: "GET".to_string(),
                url: req.url,
                headers: req.headers,
                body: None,
                body_base64: None,
                timeout_ms: req.timeout_ms,
                follow_redirects: true,
                max_response_bytes: req.max_response_bytes,
            })
            .await?;
        let byte_count = outcome.body.len();
        let body_base64 = base64::engine::general_purpose::STANDARD.encode(&outcome.body);
        Ok(BinaryResponse {
            status: outcome.status,
            final_url: outcome.final_url,
            headers: outcome.headers,
            body_base64,
            byte_count,
            truncated: outcome.truncated,
        })
    }

    #[tool(
        description = "Perform an arbitrary HTTP request and return status, headers, and body. Use this when you need full control over method, headers, body, redirects, or timeout."
    )]
    async fn fetch(
        &self,
        Parameters(req): Parameters<FetchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = self.fetch_inner(req).await.map_err(to_mcp_err)?;
        json_tool_result(&resp)
    }

    #[tool(
        description = "GET a URL and parse the response as JSON. Convenience for the common case of fetching a JSON API."
    )]
    async fn get_json(
        &self,
        Parameters(req): Parameters<GetJsonRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = self.get_json_inner(req).await.map_err(to_mcp_err)?;
        json_tool_result(&resp)
    }

    #[tool(
        description = "POST a JSON body to a URL and parse the response as JSON. Sets `Content-Type: application/json` automatically."
    )]
    async fn post_json(
        &self,
        Parameters(req): Parameters<PostJsonRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = self.post_json_inner(req).await.map_err(to_mcp_err)?;
        json_tool_result(&resp)
    }

    #[tool(
        description = "GET a URL and return the response body as base64. Use for binary content like images, archives, or PDFs."
    )]
    async fn download_binary(
        &self,
        Parameters(req): Parameters<DownloadBinaryRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = self.download_binary_inner(req).await.map_err(to_mcp_err)?;
        json_tool_result(&resp)
    }
}

#[tool_handler]
impl ServerHandler for NetworkProxy {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "HTTP egress proxy. Use `fetch` for full HTTP control, or the convenience tools \
                 `get_json`, `post_json`, and `download_binary`."
                    .to_string(),
            )
    }
}

fn outcome_to_fetch_response(outcome: FetchOutcome) -> FetchResponse {
    match String::from_utf8(outcome.body) {
        Ok(text) => FetchResponse {
            status: outcome.status,
            final_url: outcome.final_url,
            headers: outcome.headers,
            body: Some(text),
            body_base64: None,
            truncated: outcome.truncated,
        },
        Err(err) => {
            let bytes = err.into_bytes();
            FetchResponse {
                status: outcome.status,
                final_url: outcome.final_url,
                headers: outcome.headers,
                body: None,
                body_base64: Some(base64::engine::general_purpose::STANDARD.encode(&bytes)),
                truncated: outcome.truncated,
            }
        }
    }
}

fn outcome_to_json_response(outcome: FetchOutcome) -> anyhow::Result<JsonResponse> {
    let json: serde_json::Value = serde_json::from_slice(&outcome.body)
        .map_err(|e| anyhow::anyhow!("response body is not valid JSON: {e}"))?;
    Ok(JsonResponse {
        status: outcome.status,
        final_url: outcome.final_url,
        headers: outcome.headers,
        json,
    })
}

fn with_default_accept_json(mut headers: BTreeMap<String, String>) -> BTreeMap<String, String> {
    headers
        .entry("accept".to_string())
        .or_insert_with(|| "application/json".to_string());
    headers
}

fn json_tool_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string(value)
        .map_err(|e| McpError::internal_error(format!("serializing tool result: {e}"), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn to_mcp_err(err: anyhow::Error) -> McpError {
    McpError::internal_error(format!("{err:#}"), None)
}
