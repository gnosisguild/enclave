# Enclave Smart Contracts

## Importing the contracts, interfaces or types

To install, run

```sh
pnpm add @enclave-e3/contracts
```

If writing a new E3 program, you can import the necessary interfaces by writing
something similar to:

```solidity
import {
    IE3Program,
} from "@enclave-e3/contracts/contracts/interfaces/IE3Program.sol";

contract MockE3Program is IE3Program {...}
```

[Check out the E3 mock for an example](./contracts/test/MockE3Program.sol)

## To deploy

```sh
pnpm deploy --network [network]
```

This will add the deployment information to the `./ignition/deployments`
directory, as well as to the `deployed_contracts.json` file.

Be sure to configure your desired network in `hardhat.config.ts` before
deploying.

## Localhost deployment

If you are running Enclave locally, you can first start a local hardhat (or
Anvil) node, then deploy the contracts using the following commands:

```sh
pnpm hardhat node
pnpm clean:deployments
pnpm deploy:mocks --network localhost
```

This will ensure that you are a local node running, as well as that there are no
conflicting deployments stored in localhost.

## Registering a Ciphernode

To add a ciphernode to the registry, run

```sh
pnpm ciphernode:add --network [network] --ciphernode-address [address]
```

To request a new committee, run

```sh
pnpm run hardhat committee:new --network [network]
```

To publish the public key of a committee, run

```sh
pnpm run hardhat --network [network] committee:publish --e3-id [e3-id] --nodes [node address],
[node address] --public-key [publickey]
```

To activate an E3, run

```sh
pnpm run hardhat --network [network] e3:activate --e3-id [e3-id]
```

To publish an input for an active E3, run

```sh
pnpm run hardhat --network [network] e3:publishInput --e3-id [e3-id] --data [input data]
```
