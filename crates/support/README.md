This is a project to build the support container to allow risc0 to be run within docker by `enclave program start`

The conatiner is built using the github workflow [here](../../.github/workflows/support-docker.yml)
You can also build it locally by using the `./scripts/build.sh` script.

To develop on this you should log into the container by running `./scripts/dev.sh` and then you can run `cargo build --locked` with access to the risc0 environment.

```mermaid
graph TD
    subgraph N["e3-support-scripts"]
        A["enclave program start"]
        AA["./.enclave/support/ctl/start"]
        A --> AA
    end
    M["instigator"] --"http\:\/\/localhost\:13151\/run_compute (cb in payload)"--> D
    D --"http\:\/\/someurl.com"--> O["callback server receives results"]
    AA --listen on port 13151--> D
    subgraph C["e3-support (container)"]
        D["app"]
        E["host"]
        F["types"]
        G["compute-provider"]
        H["methods (risc0)"]
        I["guest (risc0)"]
        J["user-program"]

        D --> E
        D --> F
        D --> G

        E --> H
        E --> G
        E --> J

        H --> I

        I --> G
        I --> J
    end
```

NOTE: This is outside of the main workspace because it needs to be run within it's own context in order to isolate risc0.

NOTE: We are attempting to isolate risc0 - it is anticipated that we will have to use feature flags to tody this up so that we can compile more of the code and enable rust-analyzer to work outside of the risc0 environment for this project.

**NOTE: currently this is an open relay which is a known issue**
