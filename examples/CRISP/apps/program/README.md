# Program

This module does the following:

- Listens for Enclave `events` from the blockchain
- Manages `risc0` computations
- Persist processed `events`

This is the program component for our architecture here.

```mermaid
graph TD
  subgraph ec2_1["Docker swarm cluster"]
    compute_engine["program"]--store completed--> cpdb[(events)]
    server["server"] --> db
    db[(DB)]
    client --"proofs/get_data"--> server

  end
  compute_engine ---> bonsai
  compute_engine -."listen for events".-> evm
  compute_engine -- "publishCiphertextOutput(proof)" ---> evm

  bonsai["bonsai (risc0)"]
  server -. "listen for events" ..-> evm
  server --".publishInput()"--> evm
  subgraph evm
    esol1["Enclave.sol"]
    csol1["CRISP.sol"]
  end
```
