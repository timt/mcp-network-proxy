# mcp-network-proxy

An MCP server that gives [Claude Desktop](https://claude.ai/download) and [Cowork](https://www.anthropic.com/research/cowork) access to the internet from your machine.

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

3. **Add it to Claude Desktop** — pick one:

   **Option A — Connectors UI (run the server yourself)**

   Start the server in a terminal and leave it running:

   ```sh
   mcp-network-proxy http --bind 127.0.0.1:8080
   ```

   Then in Claude Desktop go to **Settings → Connectors → Add custom connector** and enter:

   - **Name:** `network-proxy`
   - **Remote MCP server URL:** `http://127.0.0.1:8080/mcp`

   **Option B — JSON config (Desktop manages the server for you)**

   Add this to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

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

## Test it

Create a Cowork task with this prompt:

> Use the `get_json` network-proxy tool to fetch `https://httpbin.org/get` and show me the response. Tell me which tool you used.

If it works, you'll see the httpbin JSON response and confirmation that it used the `get_json` tool from `network-proxy`.

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
