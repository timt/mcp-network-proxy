use std::collections::BTreeMap;

use rmcp::schemars;
use serde::{Deserialize, Serialize};

fn default_method() -> String {
    "GET".to_string()
}

fn default_timeout_ms() -> u64 {
    30_000
}

fn default_max_bytes() -> usize {
    10 * 1024 * 1024
}

fn default_true() -> bool {
    true
}

/// Request to perform an arbitrary HTTP fetch.
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct FetchRequest {
    /// HTTP method, e.g. "GET", "POST". Defaults to "GET".
    #[serde(default = "default_method")]
    pub method: String,
    /// Absolute URL to fetch.
    pub url: String,
    /// Optional request headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Optional request body as a UTF-8 string. Mutually exclusive with `body_base64`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Optional base64-encoded request body. Mutually exclusive with `body`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_base64: Option<String>,
    /// Per-request timeout in milliseconds. Defaults to 30000.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Whether to follow redirects. Defaults to true.
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    /// Maximum response body size in bytes. Defaults to 10 MiB.
    #[serde(default = "default_max_bytes")]
    pub max_response_bytes: usize,
}

/// Result of a fetch.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FetchResponse {
    pub status: u16,
    pub final_url: String,
    pub headers: BTreeMap<String, String>,
    /// UTF-8 body if the response was decodable as UTF-8 text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Base64 body, set when the response was not valid UTF-8.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_base64: Option<String>,
    /// True if the response was truncated at `max_response_bytes`.
    pub truncated: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct GetJsonRequest {
    /// Absolute URL to GET.
    pub url: String,
    /// Optional request headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Per-request timeout in milliseconds. Defaults to 30000.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct JsonResponse {
    pub status: u16,
    pub final_url: String,
    pub headers: BTreeMap<String, String>,
    pub json: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct PostJsonRequest {
    /// Absolute URL to POST to.
    pub url: String,
    /// JSON value to send as the request body.
    pub json: serde_json::Value,
    /// Optional request headers. `Content-Type: application/json` is set automatically.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Per-request timeout in milliseconds. Defaults to 30000.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
pub struct DownloadBinaryRequest {
    /// Absolute URL to download.
    pub url: String,
    /// Optional request headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Per-request timeout in milliseconds. Defaults to 30000.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Maximum response body size in bytes. Defaults to 10 MiB.
    #[serde(default = "default_max_bytes")]
    pub max_response_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BinaryResponse {
    pub status: u16,
    pub final_url: String,
    pub headers: BTreeMap<String, String>,
    /// Base64-encoded response body.
    pub body_base64: String,
    pub byte_count: usize,
    pub truncated: bool,
}
