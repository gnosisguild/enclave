# e3-init

Project scaffolding behind `interfold init`: download a template, copy/filter its files, rewrite
dependencies for that checkout, initialize the project, and optionally install JS dependencies. It
does not define the generated application's protocol behavior.

## Create a project

With the [CLI](../cli/README.md) installed, run outside an existing Interfold project:

```sh
interfold init my-project
interfold init my-project --skip-install
```

An explicit template uses a Git URL with a ref and repository subpath:

```sh
interfold init my-project --template 'git+https://github.com/theinterfold/interfold.git#main:templates/default'
```

The default template ref follows the CLI package version. Generated Rust Git dependencies pin the
downloaded checkout's commit; npm versions come from that checkout's packages. See the
[default template](../../templates/default/README.md) for the resulting project's workflow.

The library entry is `execute(location, template, skip_cleanup, skip_install, verbose).await`. Its
two optional inputs are the destination `PathBuf` and template string. Normal use needs Git/network
access and the package manager selected by the template; `skip_install` skips JavaScript dependency
installation, not template acquisition.

The destination must be empty. Installation creates/removes scratch content and normally cleans up
on failure; `--skip-cleanup` leaves it for diagnosis. The current shared scratch location also means
concurrent installs can collide. Use a disposable destination when trying templates.

Run `cargo test --locked -p e3-init` for parsing/file transformations. A full generated-project
install is a separate integration exercise with the prerequisites above.
