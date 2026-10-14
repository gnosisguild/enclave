# Edge evidence for the hand-drawn blueprint diagrams

Each JSON file here answers a list of edge claims. For every claim:

```json
{
  "id": "A1",
  "supported": true,
  "refs": ["path@line@token"],
  "line_text": ["the cited line, trimmed"],
  "note": "one short sentence"
}
```

- `path` is relative to the repository root. `line` comes from `grep -n`, not from a guess. `token`
  is an exact, distinctive part of that line, with no `@`.
- `supported: false` means that the code does not do what the claim says. Then `note` says what the
  code does.
- Point at code (`.rs`, `.sol`, `.nr`, `.ts`), never at docs.
