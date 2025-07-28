---
type: system
description: Runtime support systems for an Enclave Application
---

## `=this.file.name`

`=this.description`

```mermaid
flowchart TB
    RDR["Risc0 Docker Runner"]
    DR["Dev Runner"]
    PS["E3ProgramServer"]

    RDR --> PS
    DR --> PS

    RDR:::internal-link
    DR:::internal-link
    PS:::internal-link

    click RDR "https://github.com/gnosisguild/enclave/tree/main/crates/support/Risc0 Docker Runner.md"
    click DR "https://github.com/gnosisguild/enclave/tree/main/crates/support-scripts/Dev Runner.md"
    click PS "https://github.com/gnosisguild/enclave/tree/main/path/to/E3ProgramServer.md"
```

This package is designed so that the following are installed in an enclave template in order to run programs within an enclave project.

| Path                     | Packge                                                                                  |
| ------------------------ | --------------------------------------------------------------------------------------- |
| `./.enclave/support/dev` | Compile and run the program in a webserver                                              |
| `./.enclave/support/ctl` | Copmile and run the program using the docker risc0 runner (see [[Risc0 Docker Runner]]) |

These commands are run from `enclave program start --dev` or `enclave program start`
