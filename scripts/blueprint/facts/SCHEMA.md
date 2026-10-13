# Facts schema

Each file here describes a group of units (crates, contracts, circuits, packages) for an engineer
who is learning this codebase. An agent or a person writes a file from the code. The blueprint shows
the facts in its generated diagrams, and its build checks every reference.

Read the code of the repository. Change no code.

Your job: describe what each unit (crate, contract, circuit, package) does, for an engineer who is
learning this codebase. Every fact must come from the code you read. When you are not sure, put the
claim in `unverified` instead of stating it.

## How to work

- Read selectively. Start with `Cargo.toml`, `src/lib.rs` (or `main.rs`) and the module list. Then
  use `grep -rn` for `impl Actor for`, `impl Handler<`, `#[derive(Message`, `pub enum`,
  `pub struct`, `pub trait`, `publish(`, `subscribe`, `EventType::`, `sled`, `Persistable`,
  `Repository`, `#[test]`. Read the matching sections only.
- `agent/CRATES_ARCHITECTURE.md` and `agent/flow-trace/*.md` describe the architecture. You may read
  them for orientation, but every `ref` must point at code (`.rs`, `.sol`, `.nr`, `.ts`), not at a
  `.md` file.
- Get line numbers from `grep -n`. Never guess a line number.

## Reference format (`ref`)

A reference is one string: `path@line@token`

- `path`: relative to the repository root, for example `crates/sortition/src/sortition.rs`.
- `line`: the 1-based line number, from `grep -n`.
- `token`: an exact substring of that line. Use a distinctive identifier, for example
  `impl Handler<E3Requested> for Sortition` or `pub struct ThresholdKeyshare`. It must not contain
  `@`. The build accepts the token up to three lines from the cited line, uses the nearest copy of
  the token when the line moved further, and drops the entry with a warning when the token is gone.
  `check.mjs --fix` rewrites the line number to the exact line.

## Output

Write exactly one file: the path your task names, in `scripts/blueprint/facts/`. Write valid JSON
(no comments, no trailing commas). Then validate it with
`node -e 'JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"))' <file>` and fix it until
it parses. Write no other file.

```json
{
  "group": "<the group name from your task>",
  "units": [
    {
      "id": "e3-sortition",
      "kind": "crate",
      "path": "crates/sortition",
      "one_line": "One sentence, at most 25 words: what it does.",
      "explain": "2-4 short sentences for a newcomer: the problem it solves, how it fits into the node or protocol, and what would break without it.",
      "analogy": "One sentence: a physical-world analogy that is true to how it works (for example: a lottery drum that draws committee members, weighted by tickets).",
      "actors": [
        {
          "name": "Sortition",
          "role": "what this actor owns and decides",
          "ref": "path@line@token"
        }
      ],
      "consumes": [{ "name": "E3Requested", "by": "Sortition", "ref": "path@line@token" }],
      "emits": [
        { "name": "CiphernodeSelected", "by": "CiphernodeSelector", "ref": "path@line@token" }
      ],
      "state": [
        { "name": "what is stored", "where": "disk | memory | chain", "ref": "path@line@token" }
      ],
      "key_types": [{ "name": "TypeName", "what": "one line", "ref": "path@line@token" }],
      "read_first": [
        { "why": "the entry point: where the crate is wired up", "ref": "path@line@token" }
      ],
      "rust_patterns": [
        { "pattern": "actix-handler", "note": "what this example shows", "ref": "path@line@token" }
      ],
      "depends_on": ["e3-events", "e3-data"],
      "tests": "where the tests are and the command, for example: cargo test -p e3-sortition",
      "unverified": ["any claim you could not confirm in code"]
    }
  ]
}
```

Field rules:

- `kind`: `crate`, `contract`, `circuit`, or `package`.
- `actors`, `consumes`, `emits`, `state`: empty arrays when the unit has none (for example a pure
  library). For a contract, use `consumes` for the external functions that change state (`name` is
  the function) and `emits` for its Solidity events. For a circuit, use `consumes` for the inputs
  (`name`: `public: <input>` or `private: <input>`) and put what it proves in `explain`.
- `read_first`: 2-5 references, in the order a newcomer should read them.
- `rust_patterns`: 2-6 clear examples, only for Rust units. Use these names only:
  `result-question-mark`, `anyhow-context`, `thiserror-enum`, `enum-match`, `option-combinators`,
  `newtype`, `impl-from`, `trait-def`, `trait-impl`, `generic-where`, `trait-object-dyn`,
  `impl-trait-arg`, `closure`, `iterator-chain`, `ownership-clone`, `borrow-ref`, `lifetime`,
  `arc-shared`, `async-await`, `tokio-spawn`, `channel`, `tokio-select`, `actix-actor`,
  `actix-handler`, `actix-message`, `actix-recipient`, `actix-timer`, `serde-derive`,
  `builder-pattern`, `macro-rules`, `derive-macro`, `unit-test`, `async-test`, `let-else`,
  `pattern-destructuring`, `const-generic`.
- `depends_on`: workspace crates only (names that start with `e3-` or `interfold`), from the unit's
  `Cargo.toml` `[dependencies]` (not dev-dependencies).
- Describe the current code only. Do not speculate about bugs or security weaknesses.
- Keep each unit under about 60 lines of JSON. Prefer fewer, correct entries over many guesses.
