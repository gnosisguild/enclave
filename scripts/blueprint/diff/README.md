# Blueprint diff (experimental)

This folder draws the architecture change of a branch on the blueprint, and a workflow posts it on
each pull request. It is an experiment: [Turn it off](#turn-it-off) and [Remove it](#remove-it) say
how to stop it.

## Try it

```bash
node scripts/blueprint/diff/diff.mjs                    # your branch against its merge base with main
node scripts/blueprint/diff/diff.mjs v0.18.0 v0.19.0    # any two refs or folders
```

The command writes `scripts/blueprint/out/interfold-blueprint-diff.html`: the blueprint of your
branch with the change in colour. Green is new, red and dashed is removed, and yellow is changed. A
number on a box counts the changes inside it. **Before** and **After** (or `B`) switch between the
two versions, and **Changes** (or `C`) lists every change and opens its diagram.

It compares the crates and their dependencies, the actors, the messages that each actor handles, the
actor addresses that each actor holds, the contract functions and events, the circuits, and every
box and arrow of the diagrams. Each version is read with its own `scripts/blueprint`, so a change to
`LAYERS` or `CONTRACTS` counts too. It builds nothing.

## On pull requests

The `Blueprint Diff` workflow (`.github/workflows/blueprint-diff.yml`) runs on every pull request
from this repository and keeps one comment up to date: pictures of the changed diagrams, the counts,
every change, and the page as a download. A comment shows an image only from a URL, so
`previews.mjs` stores the pictures on the `blueprint-previews` branch, one folder per pull request:
about 0.6 MB for each push that changes the architecture. A pull request without an architecture
change gets no comment. A pull request from a fork gets none: its token can neither store the
pictures nor post the comment.

## Accuracy

On the 40 pull requests merged from #1996 to #2236, it found all 36 changes of the kinds above and
reported none that was not real. It reads patterns in the code and compiles nothing, so it does not
see what a function does inside, events published on the bus, Solidity view, pure, internal and
private functions, interfaces and mocks, contracts and circuits that `CONTRACTS` and `CIRCUITS` do
not list, the inside of the SDK and of CRISP, config, CLI flags, network messages, or test code.

## Turn it off

Set the repository variable `BLUEPRINT_DIFF` to `off` (Settings → Secrets and variables → Actions →
Variables). The workflow then skips every pull request. Delete the variable to turn it back on.

## Remove it

1. Delete this folder and `.github/workflows/blueprint-diff.yml`.
2. Delete the pictures: `git push origin --delete blueprint-previews`. The pictures in old pull
   request comments then go blank.
3. Remove the two pointers to this folder, in `scripts/blueprint/README.md` and `scripts/README.md`.

These stay and are safe to keep: `lib/page.mjs` (the build uses it), the `BLUEPRINT_ROOT` variable,
`members()` and the `unmapped` option in `lib/inventory.mjs`, and the `export` on `evName` in
`generate.mjs`. `build.mjs` and `check.mjs` work without this folder.

## Files

- `diff.mjs`: reads two versions, compares them, and writes the page, the pictures and the comment.
- `view.js` and `view.css`: the change view on the blueprint page.
- `previews.mjs`: stores the pictures on the `blueprint-previews` branch.
