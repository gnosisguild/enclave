# Program

This module does the following:

- Listens for Enclave `events` from the blockchain
- Manages `risc0` computations
- Persist processed `events`

This is the program component for our architecture here.

```mermaid
graph TD
  subgraph ec2_1["NODE"]
    server["server"] --> db
    db[(DB)]
    client --"HTTP"--> server
    server --HTTP--> program
  end
  subgraph thirdparty["3rd PARTY"]
    bonsai
  end
  program ---> bonsai

  bonsai["bonsai (risc0)"]

  server --"publishInput()"--> evm
  subgraph evm["EVM"]
    esol1["Enclave Contracts"]
    csol1["CRISP Contracts"]
  end
  server -. "WebSocket listener" .-> evm
```
