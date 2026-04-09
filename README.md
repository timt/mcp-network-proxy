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

## Using it with Claude Cowork

Cowork tasks run inside a sandbox that can't reach the open internet. The trick is to run `mcp-network-proxy` on your own machine and let Cowork talk to it as a remote MCP server. Because Cowork runs on Anthropic's infrastructure, "localhost" on your laptop isn't reachable from the task — you need a public URL that tunnels back to your machine.

End-to-end setup:

1. **Install and run the proxy locally** in HTTP mode:

   ```sh
   cargo install --path .
   mcp-network-proxy http --bind 127.0.0.1:8080
   ```

   Keep it bound to `127.0.0.1` — never expose it directly on `0.0.0.0`. The tunnel does the exposing.

2. **Expose it via a tunnel** so Cowork can reach it. Any of these work; pick whichever you already use:

   - **Cloudflare Tunnel** (`cloudflared tunnel --url http://127.0.0.1:8080`) — gives you a public `https://*.trycloudflare.com` URL. Front it with Cloudflare Access for auth.
   - **Tailscale Funnel** (`tailscale funnel 8080`) — public HTTPS URL on your tailnet, no extra account needed if you already have Tailscale.
   - **ngrok** (`ngrok http 8080`) — quickest to start; use a paid plan + basic auth or an OAuth edge to keep it private.

   **Put authentication in front of the tunnel.** Without it, anyone on the internet who finds the URL can make HTTP requests from your machine — see the security warning above. This is an SSRF primitive; treat the tunnel URL like a credential.

3. **Register it as an MCP server in Cowork.** In Cowork's settings, add a remote MCP server pointing at your tunnel URL (with `/mcp` on the end):

   ```json
   {
     "mcpServers": {
       "network-proxy": {
         "url": "https://your-tunnel-url.example.com/mcp"
       }
     }
   }
   ```

4. **Nudge the model in your task prompt** if needed: *"Use the `network-proxy` MCP tools (`fetch`, `get_json`, `post_json`, `download_binary`) for any HTTP requests."* Cowork's sandbox sometimes makes it ambiguous whether the model should reach for these tools, so being explicit helps.

5. **Keep the proxy running.** While your task is in flight, the proxy and the tunnel both have to be up on your machine. On macOS, the cleanest way to make this survive logouts and reboots is a launchd user agent in `~/Library/LaunchAgents/` that runs `mcp-network-proxy http --bind 127.0.0.1:8080` at login.

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
