use std::{collections::BTreeMap, time::Duration};

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use bytes::BytesMut;
use futures_util::StreamExt;
use reqwest::{header::HeaderMap, redirect, Method};

use crate::types::FetchRequest;

/// HTTP execution layer used by every tool.
///
/// Holds two pre-built `reqwest::Client`s — one that follows redirects, one
/// that doesn't — so per-request `follow_redirects` doesn't require rebuilding
/// a client on every call.
pub struct HttpClient {
    follow: reqwest::Client,
    no_follow: reqwest::Client,
}

/// Outcome of a fetch — kept transport-agnostic so callers can shape it into
/// whatever response type they need.
pub struct FetchOutcome {
    pub status: u16,
    pub final_url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub truncated: bool,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let user_agent = concat!("mcp-network-proxy/", env!("CARGO_PKG_VERSION"));
        let follow = reqwest::Client::builder()
            .user_agent(user_agent)
            .redirect(redirect::Policy::limited(10))
            .build()
            .context("building redirect-following reqwest client")?;
        let no_follow = reqwest::Client::builder()
            .user_agent(user_agent)
            .redirect(redirect::Policy::none())
            .build()
            .context("building non-redirect reqwest client")?;
        Ok(Self { follow, no_follow })
    }

    pub async fn execute(&self, req: FetchRequest) -> Result<FetchOutcome> {
        if req.body.is_some() && req.body_base64.is_some() {
            return Err(anyhow!("`body` and `body_base64` are mutually exclusive"));
        }

        let method = Method::from_bytes(req.method.as_bytes())
            .with_context(|| format!("invalid HTTP method: {}", req.method))?;

        let client = if req.follow_redirects {
            &self.follow
        } else {
            &self.no_follow
        };

        let mut builder = client
            .request(method, &req.url)
            .timeout(Duration::from_millis(req.timeout_ms));

        for (k, v) in &req.headers {
            builder = builder.header(k, v);
        }

        if let Some(text) = req.body {
            builder = builder.body(text);
        } else if let Some(b64) = req.body_base64 {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64.as_bytes())
                .context("decoding body_base64")?;
            builder = builder.body(bytes);
        }

        let response = builder.send().await.context("sending HTTP request")?;
        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let headers = headers_to_map(response.headers());

        let max = req.max_response_bytes;
        let mut buf = BytesMut::with_capacity(std::cmp::min(max, 64 * 1024));
        let mut truncated = false;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("reading response body chunk")?;
            if buf.len() + chunk.len() > max {
                let take = max - buf.len();
                buf.extend_from_slice(&chunk[..take]);
                truncated = true;
                break;
            }
            buf.extend_from_slice(&chunk);
            if buf.len() == max {
                // Possibly more bytes coming; mark as potentially truncated by
                // peeking the next chunk only if it exists.
                if let Some(next) = stream.next().await {
                    let _ = next.context("reading response body chunk")?;
                    truncated = true;
                }
                break;
            }
        }

        Ok(FetchOutcome {
            status,
            final_url,
            headers,
            body: buf.to_vec(),
            truncated,
        })
    }
}

fn headers_to_map(headers: &HeaderMap) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (name, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            out.insert(name.as_str().to_string(), v.to_string());
        }
    }
    out
}
