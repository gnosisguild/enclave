# @enclave-e3/mcp

MCP server for [The Interfold](https://theinterfold.com) documentation. Allows AI assistants to
answer questions about The Interfold by fetching content directly from
[docs.theinterfold.com](https://docs.theinterfold.com).

## Requirements

- Node.js **>=18.20.0** — required for ESM JSON import attributes, global `fetch`, and top-level
  await used by the `enclave-mcp` CLI.

## Tools

| Tool          | Description                                                                        |
| ------------- | ---------------------------------------------------------------------------------- |
| `list_docs`   | List all available documentation pages                                             |
| `read_doc`    | Read a specific page by slug (e.g. `introduction`, `ciphernode-operators/running`) |
| `search_docs` | Search for a keyword across all pages                                              |

## Integration

### Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or
`%APPDATA%\Claude\claude_desktop_config.json` (Windows):

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
      - '@enclave-e3/mcp'
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

- _"What is an E3?"_
- _"How do I run a ciphernode?"_
- _"Explain the Interfold architecture"_
- _"Search the docs for threshold encryption"_

## License

LGPL-3.0-only
