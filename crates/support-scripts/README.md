# e3-support-scripts

The backend of `interfold program compile`, `interfold program start` and
`interfold program cache purge`.

The `compile` and `start` bash scripts are in `dev/` and `openvm/`. The `interfold` CLI embeds them
and runs them in the project directory, so a project does not hold a copy.

| Script           | What it runs                                                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `dev/compile`    | `cargo build --bin program-runner`                                                                                               |
| `dev/start`      | `cargo run --bin program-runner -- dev`                                                                                          |
| `openvm/compile` | The OpenVM guest, its keys, the receipt identity and the worker configuration, then `cargo build --release --bin program-runner` |
| `openvm/start`   | `cargo run --release --bin program-runner -- openvm`                                                                             |

`program-runner` is the binary of the project's `runner/` crate. It calls `e3_program_runner::run`
(`crates/program-runner`), and the first argument selects the `dev` or the `openvm` mode.
