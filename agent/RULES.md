# Enclave — Agent Rules

These rules apply to any LLM agent working on this codebase. Tool-specific config files
(.cursor/rules/enclave.mdc, CLAUDE.md, etc.) should reference this file rather than duplicating its
content.

## Project Structure

- `crates/` — Rust workspace: CLI, actors, crypto, networking, FHE, EVM integration
- `packages/` — TypeScript: Solidity contracts (`enclave-contracts`), SDK, React, MCP server
- `circuits/` — ZK proof circuits
- `tests/` — Integration tests
- `agent/` — LLM context documentation

## Flow-Trace Documentation (`agent/flow-trace/`)

The `agent/flow-trace/` directory contains detailed protocol documentation that traces the complete
lifecycle of the Enclave protocol — from node registration through DKG, computation, decryption,
failure handling, and deactivation.

### When to consult

Read the relevant flow-trace file **before** modifying code in any of these areas:

| Area                                                                                 | File to read                        |
| ------------------------------------------------------------------------------------ | ----------------------------------- |
| CLI commands (`setup`, `register`, `activate`, `status`), on-chain registration, IMT | `01_REGISTRATION.md`                |
| ENCL bonding, USDC tickets, activation thresholds, exit queue                        | `02_TOKENS_AND_ACTIVATION.md`       |
| E3 requests, fee payment, committee selection, sortition, ticket submission          | `03_E3_REQUEST_AND_COMMITTEE.md`    |
| DKG, BFV keygen, ZK proofs (C0–C7), Shamir shares, key aggregation, decryption       | `04_DKG_AND_COMPUTATION.md`         |
| Timeouts, `markE3Failed`, refunds, accusations, slashing (Lane A/B)                  | `05_FAILURE_REFUND_SLASHING.md`     |
| Deactivation, deregistration, E3 completion, node shutdown, sync/restart             | `06_DEACTIVATION_AND_COMPLETION.md` |

Always start from `00_INDEX.md` if unsure which file is relevant.

### How to navigate

1. Open `agent/flow-trace/00_INDEX.md` — it has a topic table and end-to-end flow summaries
2. Find the file that covers your area of interest
3. Each file traces the flow call-by-call with file paths, function names, and event names
4. The index also contains a "Verified Bugs & Protocol Concerns" section — check it before assuming
   current behavior is correct

### When to update

Update flow-trace docs **in the same PR** when any of these happen:

- A contract function signature, event, or state variable changes
- An actor's message handling or event routing changes
- A CLI command's behavior or arguments change
- A ZK circuit or proof pipeline step is added, removed, or reordered
- A timeout, threshold, or fee calculation formula changes
- A bug listed in "Verified Bugs & Protocol Concerns" is fixed or a new one is found

### How to update

- Edit the specific file that covers the changed area — keep changes scoped
- If a change spans multiple files, update all affected files
- Update `00_INDEX.md` only when adding/removing/renaming a file, or when the end-to-end flow
  summaries or the contract interaction map change
- Preserve the existing format: step-by-step traces with `File:` references pointing to actual
  source paths
- Keep the "Verified Bugs" table in `00_INDEX.md` current — mark fixed bugs, add new ones
- Do NOT rewrite entire files for small changes — surgical edits only

### Organization rules

- Files are numbered sequentially (`01_`, `02_`, ...) following the protocol lifecycle order
- Each file covers one logical phase of the protocol
- To add a new phase, use the next available number and add it to the index table
- File names use `SCREAMING_SNAKE_CASE` after the number prefix
