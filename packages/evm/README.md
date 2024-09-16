# Enclave EVM

## To deploy

```
yarn deploy --network [network]
```

This will add the deployment information to the `./deployments` directory.

Be sure to configure your desired network in `hardhat.config.ts` before
deploying.

## Registering a Ciphernode

To add a ciphernode to the registry, run

```
yarn ciphernode:add --network [network] --ciphernode-address [address]
```

To remove a ciphernode, run

```
yarn ciphernode:remove --network [network] --ciphernode-address [address]
```
