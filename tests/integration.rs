use std::{collections::BTreeMap, sync::Arc};

use base64::Engine;
use mcp_network_proxy::{
    http_client::HttpClient,
    proxy::NetworkProxy,
    types::{DownloadBinaryRequest, FetchRequest, GetJsonRequest, PostJsonRequest},
};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn proxy() -> NetworkProxy {
    NetworkProxy::new(Arc::new(HttpClient::new().expect("client")))
}

fn fetch_req(url: String) -> FetchRequest {
    FetchRequest {
        method: "GET".to_string(),
        url,
        headers: BTreeMap::new(),
        body: None,
        body_base64: None,
        timeout_ms: 5_000,
        follow_redirects: true,
        max_response_bytes: 1024 * 1024,
    }
}

#[tokio::test]
async fn fetch_get_returns_status_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hi there"))
        .mount(&server)
        .await;

    let proxy = proxy();
    let resp = proxy
        .fetch_inner(fetch_req(format!("{}/hello", server.uri())))
        .await
        .expect("fetch ok");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body.as_deref(), Some("hi there"));
    assert_eq!(resp.body_base64, None);
    assert!(!resp.truncated);
}

#[tokio::test]
async fn fetch_post_with_headers_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/echo"))
        .and(header("x-test", "yes"))
        .respond_with(ResponseTemplate::new(201).set_body_string("created"))
        .mount(&server)
        .await;

    let proxy = proxy();
    let mut headers = BTreeMap::new();
    headers.insert("x-test".to_string(), "yes".to_string());
    let resp = proxy
        .fetch_inner(FetchRequest {
            method: "POST".to_string(),
            url: format!("{}/echo", server.uri()),
            headers,
            body: Some("payload".to_string()),
            body_base64: None,
            timeout_ms: 5_000,
            follow_redirects: true,
            max_response_bytes: 1024 * 1024,
        })
        .await
        .expect("fetch ok");
    assert_eq!(resp.status, 201);
    assert_eq!(resp.body.as_deref(), Some("created"));
}

#[tokio::test]
async fn fetch_truncates_oversize_body() {
    let big = "x".repeat(8 * 1024);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_string(big))
        .mount(&server)
        .await;

    let proxy = proxy();
    let resp = proxy
        .fetch_inner(FetchRequest {
            method: "GET".to_string(),
            url: format!("{}/big", server.uri()),
            headers: BTreeMap::new(),
            body: None,
            body_base64: None,
            timeout_ms: 5_000,
            follow_redirects: true,
            max_response_bytes: 1024,
        })
        .await
        .expect("fetch ok");
    assert!(resp.truncated, "expected truncation");
    assert_eq!(resp.body.as_deref().map(|s| s.len()), Some(1024));
}

#[tokio::test]
async fn fetch_no_follow_redirects_returns_3xx() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/r"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://example.invalid/"),
        )
        .mount(&server)
        .await;

    let proxy = proxy();
    let resp = proxy
        .fetch_inner(FetchRequest {
            method: "GET".to_string(),
            url: format!("{}/r", server.uri()),
            headers: BTreeMap::new(),
            body: None,
            body_base64: None,
            timeout_ms: 5_000,
            follow_redirects: false,
            max_response_bytes: 1024 * 1024,
        })
        .await
        .expect("fetch ok");
    assert_eq!(resp.status, 302);
    assert_eq!(
        resp.headers.get("location").map(String::as_str),
        Some("https://example.invalid/")
    );
}

#[tokio::test]
async fn fetch_returns_base64_for_non_utf8() {
    let server = MockServer::start().await;
    let bytes: Vec<u8> = vec![0xff, 0xfe, 0x00, 0x01, 0x80];
    Mock::given(method("GET"))
        .and(path("/bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
        .mount(&server)
        .await;

    let proxy = proxy();
    let resp = proxy
        .fetch_inner(fetch_req(format!("{}/bin", server.uri())))
        .await
        .expect("fetch ok");
    assert_eq!(resp.body, None);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(resp.body_base64.expect("base64").as_bytes())
        .expect("base64 decode");
    assert_eq!(decoded, bytes);
}

#[tokio::test]
async fn get_json_parses_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"hello":"world","n":42}"#),
        )
        .mount(&server)
        .await;

    let proxy = proxy();
    let resp = proxy
        .get_json_inner(GetJsonRequest {
            url: format!("{}/data", server.uri()),
            headers: BTreeMap::new(),
            timeout_ms: 5_000,
        })
        .await
        .expect("get_json ok");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.json["hello"], "world");
    assert_eq!(resp.json["n"], 42);
}

#[tokio::test]
async fn post_json_round_trip() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({"k":"v"})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"ok":true}"#),
        )
        .mount(&server)
        .await;

    let proxy = proxy();
    let resp = proxy
        .post_json_inner(PostJsonRequest {
            url: format!("{}/api", server.uri()),
            json: serde_json::json!({"k":"v"}),
            headers: BTreeMap::new(),
            timeout_ms: 5_000,
        })
        .await
        .expect("post_json ok");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.json["ok"], true);
}

#[tokio::test]
async fn download_binary_returns_base64() {
    let server = MockServer::start().await;
    let bytes: Vec<u8> = (0u8..=255u8).collect();
    Mock::given(method("GET"))
        .and(path("/file.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
        .mount(&server)
        .await;

    let proxy = proxy();
    let resp = proxy
        .download_binary_inner(DownloadBinaryRequest {
            url: format!("{}/file.bin", server.uri()),
            headers: BTreeMap::new(),
            timeout_ms: 5_000,
            max_response_bytes: 1024 * 1024,
        })
        .await
        .expect("download ok");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.byte_count, 256);
    assert!(!resp.truncated);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(resp.body_base64.as_bytes())
        .expect("base64 decode");
    assert_eq!(decoded, bytes);
}
