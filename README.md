# mcp-network-proxy

An open-source [Model Context Protocol](https://modelcontextprotocol.io/) server that exposes the host's network as a set of tools.

It exists for one reason: when an MCP host runs in a sandbox with restricted network egress (for example, Claude Code on the web or **Claude Cowork**), the model can't reach the open internet. Run `mcp-network-proxy` on a machine that *does* have unrestricted network access, point the sandboxed host at it over MCP, and the model can fetch arbitrary URLs — and therefore make general network/internet calls — through the proxy.

```
   ┌─────────────────────┐         MCP          ┌──────────────────────┐
   │ Claude Code         │  ◄────────────────►  │ mcp-network-proxy    │
   │ (sandboxed host)    │   stdio or HTTP      │ (this binary)        │
   └─────────────────────┘                      └──────────┬───────────┘
                                                           │ HTTPS
                                                           ▼
                                                       Internet
```

## Security warning

**This server is intentionally open. There is no authentication.** Anyone who can reach it can use it to make HTTP requests from your machine, including to internal services on `localhost`, link-local metadata endpoints (`169.254.169.254`), or anything else your network can reach. That's an SSRF primitive.

Only run it:

- bound to `127.0.0.1` (the default), **or**
- behind a reverse proxy that enforces authentication, **or**
- on an isolated network you control.

SSRF hardening (host allow/deny lists, link-local blocking) is intentionally out of scope for the first version — if you need it, front the proxy with something that enforces it.

## Install

```sh
cargo install --path .
```

## Run

Stdio transport (for MCP hosts that spawn subprocesses):

```sh
mcp-network-proxy stdio
```

Streamable HTTP transport (for remote MCP hosts):

```sh
mcp-network-proxy http --bind 127.0.0.1:8080
```

The HTTP service is mounted at `/mcp` by default; pass `--path /custom` to change it.

Logging is controlled by `RUST_LOG` and always written to stderr (so it never corrupts the stdio JSON-RPC stream).

## Configuring an MCP client

Streamable HTTP form:

```json
{
  "mcpServers": {
    "network-proxy": {
      "url": "http://127.0.0.1:8080/mcp"
    }
  }
}
```

Stdio form:

```json
{
  "mcpServers": {
    "network-proxy": {
      "command": "mcp-network-proxy",
      "args": ["stdio"]
    }
  }
}
```

## Tools

| Tool | Purpose | Key fields |
| --- | --- | --- |
| `fetch` | Arbitrary HTTP request — full control. | `method`, `url`, `headers`, `body` / `body_base64`, `timeout_ms`, `follow_redirects`, `max_response_bytes` |
| `get_json` | GET a URL and parse the response as JSON. | `url`, `headers`, `timeout_ms` |
| `post_json` | POST a JSON value, parse the response as JSON. | `url`, `json`, `headers`, `timeout_ms` |
| `download_binary` | GET a URL and return the response body as base64. | `url`, `headers`, `timeout_ms`, `max_response_bytes` |

`fetch` returns the response body as UTF-8 text when possible and falls back to base64 when the bytes aren't valid UTF-8. Responses larger than `max_response_bytes` (default 10 MiB) are truncated, with `truncated: true` in the result.

## Smoke test

stdio:

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | mcp-network-proxy stdio
```

You should see a `tools/list` response containing `fetch`, `get_json`, `post_json`, and `download_binary`.

## Development

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Tests use [`wiremock`](https://crates.io/crates/wiremock) to stand up local HTTP fixtures and exercise the tool implementations directly.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
