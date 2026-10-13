# Blueprint

The blueprint is an interactive map of the Interfold architecture. It shows flat diagrams that open
into each other, from the whole system down to one actor and its code.

## Build and open

```bash
node scripts/blueprint/build.mjs
```

This command writes `scripts/blueprint/out/interfold-blueprint.html`. Open that file in a browser.
The build needs only Node.js. It reads the code that you have checked out, so the map shows your
current branch.

## Use

- Click a box with ⧉ to open the diagram inside it.
- Click a box with `</>`, or an arrow, to read the code behind it.
- Press Esc or **Up** to go back one level. Scroll to zoom, and drag to pan.
- Press `/` to find a box by name.

## Generated and hand-drawn parts

| Part                                                                 | Source                                                        | When the code changes                                                                                                      |
| -------------------------------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| The five top diagrams: system, contracts, circuits, ciphernode, apps | `curated.mjs` (hand-drawn)                                    | Each arrow cites code as `path@line@token`. The build uses the new line of a moved reference and drops a broken reference. |
| Crate, contract, circuit, app and actor diagrams                     | `generate.mjs`, from `lib/inventory.mjs` and `lib/wiring.mjs` | They follow the code.                                                                                                      |
| Descriptions, state and outputs in the generated diagrams            | `facts/*.json`                                                | The build uses the new line of a moved reference and drops a broken reference.                                             |
| Evidence for the hand-drawn arrows                                   | `refs/*.json`                                                 | The build checks it like the arrows.                                                                                       |

## Keep it current

```bash
node scripts/blueprint/check.mjs                   # moved and broken references, and crates missing from the map or gone
node scripts/blueprint/check.mjs --fix             # rewrite the moved line numbers
node scripts/blueprint/check.mjs --since v0.19.0   # also list the hand-drawn arrows whose code changed
```

- The build does not stop when the code changes. It uses the new line of a moved reference, drops a
  broken reference, skips a crate, contract or circuit that is gone, and prints a warning for each.
  The check is strict: it lists every problem and fails.
- No CI job runs these commands, so a moved line never blocks a pull request. Run the check before a
  release, and fix what it reports in a small pull request.
- The check finds a moved reference again by its token. When the token has other copies in the file,
  the nearest copy can be the wrong line. `--fix` lists those references: read their lines.
- When you add a crate, add its path under `crates/` (for example `zk/cli`) to `LAYERS` in
  `lib/inventory.mjs`. When you remove or rename a crate, a contract or a circuit, change it there
  too. The check lists both cases.

## Add or change a hand-drawn arrow

1. Find the line of code with `grep -n`.
2. Add `path@line@token` to the arrow in `curated.mjs`. The token is an exact part of that line, and
   it appears nowhere else in the file.
3. Run `node scripts/blueprint/check.mjs`. It fails on a reference that does not match. The build
   only warns.

## Files

- `build.mjs`: builds the page.
- `check.mjs`: checks the references against the code.
- `curated.mjs`: the five hand-drawn diagrams.
- `generate.mjs`: the generated diagrams.
- `renderer.js` and `blueprint.css`: the page in the browser.
- `lib/`: the inventory, the wiring and the reference checks.
- `facts/`: plain-English facts for each unit. `facts/SCHEMA.md` gives the format.
- `refs/`: the evidence for the hand-drawn arrows. `refs/README.md` gives the format.
- `out/`: the built page. Git ignores this directory.
