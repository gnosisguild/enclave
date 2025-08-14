
#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1
export RISC0_DEV_MODE=0

enclave program start --dev true
