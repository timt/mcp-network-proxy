# mcp-network-proxy

An MCP server that gives [Claude Desktop](https://claude.ai/download) / [Cowork](https://www.anthropic.com/research/cowork) access to the internet from your machine.

Cowork tasks run in a sandbox with no network access. This proxy lets them make HTTP requests through your local machine via MCP.

## Setup

1. **Install Rust** (if you don't have it):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Build and install:**

   ```sh
   git clone https://github.com/timt/mcp-network-proxy.git
   cd mcp-network-proxy
   cargo install --path .
   ```

3. **Add it to Claude Desktop.** Add this to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

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

   Restart Claude Desktop. It will start and stop the server automatically — no terminal needed.

   > **Note:** If `mcp-network-proxy` isn't on your `PATH`, use the full path to the binary (e.g. `/Users/you/.cargo/bin/mcp-network-proxy`).

   > **Alternative:** You can also use the Connectors UI (**Settings → Connectors → Add custom connector**), but this requires an HTTPS URL — you'd need to run the server in HTTP mode (`mcp-network-proxy http --bind 127.0.0.1:8080`) and front it with something like [ngrok](https://ngrok.com/) to provide an HTTPS endpoint.

## Test it

Create a Cowork task with this prompt:

> Use the network proxy to fetch `https://httpbin.org/get` and show me the response. Tell me which tool you used.

If it works, you'll see the JSON response and confirmation that it used the `get_json` tool from `network-proxy`.

## Tools

| Tool | Purpose |
| --- | --- |
| `fetch` | Arbitrary HTTP request with full control over method, headers, and body |
| `get_json` | GET a URL and parse the response as JSON |
| `post_json` | POST a JSON body and parse the response as JSON |
| `download_binary` | GET a URL and return the response as base64 |

## Security warning

**This server has no authentication.** It only listens on `localhost` by default. Do not expose it to the network without putting authentication in front of it.

## Development

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).
