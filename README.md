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

3. **Enable in Claude Desktop:** Go to **Settings → Connectors**, find `mcp-network-proxy`, and enable it.

Done. Claude Desktop handles starting and stopping the server automatically.

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

**This server has no authentication.** It only listens on `localhost` by default, and Claude Desktop manages its lifecycle. Do not expose it to the network without putting authentication in front of it.

## Development

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).
