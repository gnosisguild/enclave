# @enclave-e3/mcp

MCP server for [Enclave](https://enclave.gg) documentation. Allows AI assistants to answer questions about Enclave by fetching content directly from [docs.enclave.gg](https://docs.enclave.gg).

## Requirements

- Node.js **>=18.20.0** — required for ESM JSON import attributes, global `fetch`, and top-level await used by the `enclave-mcp` CLI.

## Tools

| Tool | Description |
|------|-------------|
| `list_docs` | List all available documentation pages |
| `read_doc` | Read a specific page by slug (e.g. `introduction`, `ciphernode-operators/running`) |
| `search_docs` | Search for a keyword across all pages |

## Integration

### Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "enclave-docs": {
      "command": "npx",
      "args": ["-y", "@enclave-e3/mcp"]
    }
  }
}
```

Restart Claude Desktop. The tools will be available automatically.

### VS Code (Continue)

Add a file `.continue/mcpServers/enclave.yaml` in your project:

```yaml
name: Enclave Docs
version: 0.1.0
schema: v1
mcpServers:
  - name: enclave-docs
    command: npx
    args:
      - -y
      - "@enclave-e3/mcp"
```

### Cursor

Edit `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "enclave-docs": {
      "command": "npx",
      "args": ["-y", "@enclave-e3/mcp"]
    }
  }
}
```

### Windsurf

Edit `~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "enclave-docs": {
      "command": "npx",
      "args": ["-y", "@enclave-e3/mcp"]
    }
  }
}
```

## Usage

Once configured, ask your AI assistant questions like:

- *"What is an E3 in Enclave?"*
- *"How do I run a ciphernode?"*
- *"Explain the Enclave architecture"*
- *"Search the enclave docs for threshold encryption"*

## License

LGPL-3.0-only