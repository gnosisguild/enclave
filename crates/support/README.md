This is a project to build the support container to allow risc0 to be run within docker by `enclave program start`

The conatiner is built using the github workflow [here](../../.github/workflows/support-docker.yml)
You can also build it locally by using the `./scripts/build.sh` script.

To develop on this you should log into the container by running `./scripts/dev.sh` and then you can run `cargo build` with access to the risc0 environment.

```mermaid
graph TD
  a['enclave program start']
  b['./.enclave/support/ctl/start']
  c['e3-support (container)']
  a --> b
  b --> c
```
