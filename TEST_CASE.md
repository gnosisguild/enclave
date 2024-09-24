
1. Launch evm node

```
yarn evm:node
```

2. Launch 3 ciphernodes

```
yarn ciphernode:launch --address 0x2546BcD3c84621e976D8185a91A922aE77ECEc30 --rpc "ws://localhost:8545" --enclave-contract 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 --registry-contract 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
```

```
yarn ciphernode:launch --address 0xbDA5747bFD65F08deb54cb465eB87D40e51B197E --rpc "ws://localhost:8545" --enclave-contract 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 --registry-contract 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
```

```
yarn ciphernode:launch --address 0xdD2FD4581271e230360230F9337D5c0430Bf44C0 --rpc "ws://localhost:8545" --enclave-contract 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 --registry-contract 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
```

```
yarn ciphernode:launch --address 0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199 --rpc "ws://localhost:8545" --enclave-contract 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 --registry-contract 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
```

3. Launch an Aggregator 

```
yarn ciphernode:aggregator --rpc "ws://localhost:8545" --registry-contract 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
```

4. Register nodes

```
yarn ciphernode:add --ciphernode-address 0x2546BcD3c84621e976D8185a91A922aE77ECEc30 --network localhost
```

```
yarn ciphernode:add --ciphernode-address 0xbDA5747bFD65F08deb54cb465eB87D40e51B197E --network localhost
```

```
yarn ciphernode:add --ciphernode-address 0xdD2FD4581271e230360230F9337D5c0430Bf44C0 --network localhost
```

```
yarn ciphernode:add --ciphernode-address 0x8626f6940E2eb28930eFb4CeF49B2d1F2C9C1199 --network localhost
```

5. Request Computation (WIP)

```

```
