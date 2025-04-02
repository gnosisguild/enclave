# TODO

- `./server/.env`
- `./risc0/script/config.toml`
- `./scripts/risc_deploy.sh`

Setup symlink as it means the interface in solidity within the risc0 package is out of date

```env
export ETH_WALLET_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
```

```toml
[profile.custom]
chainId = 31337
riscZeroVerifierAddress = "0x0000000000000000000000000000000000000000" # Deployed with the script. Don't set or it will be skipped.
enclaveAddress = "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0" # Based on default deployment address using local anvil node
inputValidatorAddress = "0xa513E6E4b8f2a923D98304ec87F64353C4D5C853" # Based on default deployment address using local anvil node
```

```env
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
# Based on Default Anvil Deployments (Only for testing)
ENCLAVE_ADDRESS=0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
CIPHERNODE_REGISTRY_ADDRESS=0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
NAIVE_REGISTRY_FILTER_ADDRESS=0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
E3_PROGRAM_ADDRESS=0x610178dA211FEF7D417bC0e6FeD39F05609AD788 # CRISPRisc0 Contract Address
```

inputValidator address within profile.custom needs to be adjusted to acount for excubiae
