# Enclave Ciphernode – DAppNode Package

Run an Enclave ciphernode on DAppNode.

This package wraps the `enclave` CLI in a DAppNode service so users can run a ciphernode with a
simple UI form (setup wizard) instead of hand-crafting configs and Docker commands.

## Networks

This is a **single-configuration** package: the same package can be pointed at different networks by
changing the config in the DAppNode UI.

You choose:

- `NETWORK` (e.g. `sepolia`, `mainnet`, `localhost`)
- The RPC URL [remote procedure call endpoint]
- Contract addresses and deploy blocks

All of these are set in the setup wizard or in the package config after installation.

## Files

Package layout (from the `dappnode/` directory):

```text
dappnode/
├── Dockerfile            # Builds the DAppNode image from the upstream ciphernode image
├── docker-compose.yml    # DAppNode service definition (single variant)
├── dappnode_package.json # Package metadata (name, version, links, backup, etc.)
├── setup-wizard.yml      # DAppNode UI form -> environment variables
├── entrypoint.sh         # Startup script (validates env, renders config, runs enclave)
├── config.template.yaml  # Enclave config template (filled via envsubst)
├── releases.json         # Release metadata used by DAppNode
└── avatar-default.png    # Icon shown in the DAppNode UI
```

All configuration is done via **environment variables**, wired through `docker-compose.yml` and
`setup-wizard.yml`.

## Quick Start

### For DAppNode Users

Once this package is published to the DAppStore:

1. Open your DAppNode UI (`http://my.dappnode`).
2. Search for **“Enclave Ciphernode”** and install the package.
3. The **setup wizard** will prompt you for:
   - `RPC_URL` – WebSocket RPC endpoint (e.g. `wss://ethereum-sepolia-rpc.publicnode.com`)
   - `NETWORK` – e.g. `sepolia`, `mainnet`, `localhost`
   - Contract addresses + deploy blocks
   - Node role (`ciphernode` or `aggregator`)
   - Optional keys and peers

4. Confirm and finish the installation.
5. Go to **Packages → enclave-ciphernode.public.dappnode.eth → Logs** to verify the node started
   correctly.

Until it’s in the public store, you can install it by IPFS hash:

- Build it with the SDK (see “For Developers”).
- Paste the resulting `/ipfs/...` hash into the DAppNode installer UI (“Install from IPFS hash”).

---

### For Developers

You’ll typically:

- Build the package with the DAppNode SDK.
- Install it on a DAppNode box (device or VM) from the resulting IPFS hash.
- Iterate on the entrypoint, config template, and setup wizard.

#### 1. Build the package

From the `dappnode/` directory:

```bash
cd dappnode
npx @dappnode/dappnodesdk@latest build -p remote
```

This will:

- Validate `docker-compose.yml`, `setup-wizard.yml`, and `dappnode_package.json`
- Build a multi-arch Docker image for `ciphernode.enclave-ciphernode.public.dappnode.eth`
- Upload the release to the DAppNode IPFS node
- Print an `/ipfs/<hash>` you can use to install the package

#### 2. Install on your DAppNode instance

In your browser (connected to your DAppNode):

- Open the installer URL that the SDK prints, **or**
- Go to the DAppNode UI → Installer → “Install from IPFS hash” and paste the `/ipfs/<hash>`.

Fill in the wizard fields, then install.

#### 3. Debugging and iteration

- Use the package **Logs** tab to inspect `entrypoint.sh` and `enclave` output.

- If something is wrong in the generated config, `docker exec` into the container and inspect:

  ```bash
  docker exec -it <ciphernode-container> cat /data/config.yaml
  ```

- Edit `entrypoint.sh`, `config.template.yaml`, or `setup-wizard.yml` locally, then rebuild with:

  ```bash
  npx @dappnode/dappnodesdk@latest build -p remote
  ```

- Reinstall with the new IPFS hash.

## Configuration

All runtime configuration is done via environment variables. They are:

### Core

- **`RPC_URL`** (required) WebSocket RPC endpoint for the chain (e.g.
  `wss://ethereum-sepolia-rpc.publicnode.com`).

- **`NETWORK`** Logical network name written into the Enclave config (e.g. `sepolia`, `mainnet`,
  `localhost`).

- **`NODE_ROLE`**
  - `ciphernode` – participate in threshold decryption.
  - `aggregator` – coordinate operations, requires a wallet key.

- **`ETH_ADDRESS`** Optional Ethereum address to bind the node to. Leave empty to let Enclave handle
  it.

- **`QUIC_PORT`** Internal UDP port used for QUIC [Quick UDP Internet Connections] P2P networking.
  Default in this package: `37173`.

- **`LOG_LEVEL`** One of `info`, `debug`, `trace`. Mapped internally to `-v`, `-vv`, or `-vvv` when
  calling `enclave start`.

- **`EXTRA_OPTS`** Extra flags appended to the `enclave start` CLI.

### Contracts

Used to populate the `chains[0].contracts` section in `config.yaml`:

- `ENCLAVE_CONTRACT`
- `CIPHERNODE_REGISTRY_CONTRACT`
- `BONDING_REGISTRY_CONTRACT`
- `ENCLAVE_DEPLOY_BLOCK`
- `CIPHERNODE_REGISTRY_DEPLOY_BLOCK`
- `BONDING_REGISTRY_DEPLOY_BLOCK`

These are all required in the setup wizard so that the node can index chain events from the correct
block heights.

### Secrets and keys

- **`ENCRYPTION_PASSWORD`** Optional local encryption password. If set, `entrypoint.sh` calls:
  - `enclave password set --config /data/config.yaml`

- **`NETWORK_PRIVATE_KEY`** Optional libp2p network key. If set, `entrypoint.sh` calls:
  - `enclave net set-key --config /data/config.yaml --net-keypair "$NETWORK_PRIVATE_KEY"`

- **`PRIVATE_KEY`** Optional Ethereum private key (hex). Only needed for aggregator mode. If set,
  `entrypoint.sh` calls:
  - `enclave wallet set --config /data/config.yaml --private-key "$PRIVATE_KEY"`

### Peers

- **`PEERS`** Comma-separated list of peer multiaddresses, for example:

  ```text
  /dns4/cn1/udp/37173/quic-v1,/dns4/cn2/udp/37173/quic-v1
  ```

  The entrypoint splits this on commas, trims spaces, and turns each into a `--peer` flag:

  ```bash
  enclave start ... --peer /dns4/cn1/udp/37173/quic-v1 --peer /dns4/cn2/udp/37173/quic-v1
  ```

If a variable is not set in the wizard, it still appears (with its default) in the package config
screen after installation, as per DAppNode’s env behavior.

## How It Works Internally

At container startup, `entrypoint.sh`:

1. Validates `RPC_URL` is non-empty and starts with `ws://` or `wss://`.
2. Applies sensible defaults for `NETWORK`, `QUIC_PORT`, `NODE_ROLE`, and `LOG_LEVEL`.
3. Uses `envsubst` to render `config.template.yaml` into `/data/config.yaml`, substituting:
   - node address, role, ports
   - network name and RPC URL
   - contract addresses and deploy blocks

4. Optionally programs password, network key, and wallet key via the `enclave` CLI.
5. Builds CLI args, including verbosity and `--peer` flags from `PEERS`.
6. Executes:

   ```bash
   enclave start --config /data/config.yaml ...
   ```

The state and databases live under `/data` inside the container, which is backed by the
`ciphernode_data` Docker volume and listed as a backup target in `dappnode_package.json`.

## Data & Ports

- **Data volume**: `ciphernode_data` → `/data` This is where Enclave stores its databases and state.

- **Ports**:
  - **UDP 37173** – QUIC P2P networking (host and container).

If you change `QUIC_PORT` in the config, you must also adjust the `ports:` mapping in
`docker-compose.yml` in a derived package.

## Publishing

To publish this package to the public DAppStore so others can install it:

```bash
npx @dappnode/dappnodesdk@latest publish \
  --type=<patch|minor|major> \
  --eth-provider=<your ETH RPC> \
  --content-provider=<your IPFS API> \
  --developer-address=<publisher address>
```

The SDK will guide you through signing and broadcasting the on-chain transaction that registers the
new package version.

## Links

- [Enclave Docs](https://docs.enclave.gg)
- [DAppNode Package Development – Single Configuration](https://docs.dappnode.io/docs/dev/package-development/single-configuration/)
- [DAppNode Docker Compose Reference](https://docs.dappnode.io/docs/dev/references/docker-compose/)
- [DAppNode Setup Wizard Reference](https://docs.dappnode.io/docs/dev/references/setup-wizard/)
- [Enclave GitHub Repository](https://github.com/gnosisguild/enclave)
