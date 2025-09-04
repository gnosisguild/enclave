# Enclave EVM

## Importing the contracts, interfaces or types

To install, run

```
pnpm add @enclave-e3/contracts
```

If writing a new E3 program, you can import the necessary interfaces by writing
something similar to:

```
import {
    IE3Program,
    IInputValidator,
    IDecryptionVerifier
} from "../interfaces/IE3Program.sol";

contract MockE3Program is IE3Program {...}
```

[Check out the E3 mock for an example](./contracts/test/MockE3Program.sol)

## To deploy

```
pnpm deploy --network [network]
```

This will add the deployment information to the `./deployments` directory.

Be sure to configure your desired network in `hardhat.config.ts` before
deploying.

## Registering a Ciphernode

To add a ciphernode to the registry, run

```
pnpm ciphernode:add --network [network] --ciphernode-address [address]
```

To request a new committee, run

```
pnpm run hardhat committee:new --network [network] \
```

To publish the public key of a committee, run

```
pnpm run hardhat --network [network] committee:publish --e3-id [e3-id] --nodes [node address],
[node address] --public-key [publickey] \
```

To activate an E3, run

```
pnpm run hardhat --network [network] e3:activate --e3-id [e3-id] \
```

To publish an input for an active E3, run

```
pnpm run hardhat --network [network] e3:publishInput --e3-id [e3-id] --data [input data]
```
