# e3-config

Loads node/project configuration and resolves the paths, network profile, chain connections, and
program backend consumed by the CLI and runtime. This crate interprets configuration; runtime
adapters still check chain identity, deployed releases, and peer compatibility.

## Load a named node

`e3_config::load_config(name, found_config_file, otel)` returns a resolved `AppConfig`. The optional
strings select an explicit configuration file and telemetry override. The CLI's
[loader](../cli/src/cli.rs) passes its global `--name`, `--config`, and `--otel` arguments here. The
default CLI node name is `_default`.

Configuration path selection is explicit CLI path, then ancestor search for `interfold.config.yaml`,
then the OS default configuration directory. YAML supports environment substitution.
`UnscopedAppConfig::into_scoped` resolves a selected node; the explicit-defaults variant supports
callers that need isolated directory roots.

Use `AppConfig` accessors such as `key_file()`, `db_file()`, `log_file()`, `chains()`, `network()`,
and `program()` rather than joining paths to the process working directory.
`paths_engine::PathsEngine` distinguishes project-local `.interfold` layouts, OS defaults, named
nodes, and overrides; different path fields have different bases.

## Related APIs

- `chain_config::ChainConfig` includes RPC/finality/log-range and DA settings.
- `RPC` validates HTTP(S)/WS(S) endpoints and exposes protocol conversions.
- `NetworkProfile` resolves public/local network IDs and bootstrap peers.
- `ProgramConfig` selects OpenVM or explicit development mode; removed/unknown backend settings are
  errors rather than an implicit fallback.

For operator configuration examples use the [root README](../../README.md) and
[project template](../../templates/default/README.md). Run `cargo test --locked -p e3-config` for
path, parsing, and validation tests without starting a node.
