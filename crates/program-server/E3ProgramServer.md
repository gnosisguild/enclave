---
type: lib
description: Run a webserver that conforms to the program server interface
links: "[[Support]]"
---

## `=this.file.name`

`=this.description`

## Description

Provides a library for running a compliant server to run an FHE computation.

This runs a webserver that will run the given closure locally when sent a curl request of the form:

```bash
curl -X POST "http://localhost:13151/run_compute" \
  -H "Content-Type: application/json" \
  -d '{
    "e3_id": 123,
    "params": "0x123...",
    "ciphertext_inputs": [["0x123...", 1], ["0x123...", 2]],
    "callback_url": "http://localhost:3500/callback"
  }'
```

We can use this in either a dev or risc0 environment to act as a common interface to run FHE programs