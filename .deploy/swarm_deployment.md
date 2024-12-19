# Setup `.env` vars

Copy the `.env.example` file to `.env`

```
cp .env.example .env
```

Alter the variables to reflect the correct values required for the stack:

```
export RPC_URL=wss://eth-sepolia.g.alchemy.com/v2/<SOME_API_KEY>
export SEPOLIA_ENCLAVE_ADDRESS=0xCe087F31e20E2F76b6544A2E4A74D4557C8fDf77
export SEPOLIA_CIPHERNODE_REGISTRY_ADDRESS=0x0952388f6028a9Eda93a5041a3B216Ea331d97Ab
export SEPOLIA_FILTER_REGISTRY=0xcBaCE7C360b606bb554345b20884A28e41436934
```

Pay special attention to the `TAG` and `RPC_URL` vars.

You can peruse the yaml config files for the nodes to see how the vars are used within the config.

# Secrets Setup Script

To deploy with swarm we need to set up the secrets file for our cluster.

## Run
```bash
./.deploy/copy-secrets.sh
```

## What it does
- Copies `example.secrets.json` to create `cn1/2/3` and `agg.secrets.json` files
- Skips existing files
- Warns with yellow arrows (==>) if any files are identical to the example

## Example output
```bash
Created cn1.secrets.json
Skipping cn2.secrets.json - file already exists

==> cn1.secrets.json <== # Yellow arrows indicate files that need customization
```

Remember to modify any highlighted files before use.

# Initialize docker swarm

First we need to initialize swarm.

```
docker swarm init
```

If you get an error about not being able to choose between IP addresses choose the more private IP address.

```
docker swarm init --advertise-addr 10.49.x.x
```

# Build the dockerfile image

```
./.deploy/build.sh
```

# Deploy a version to the stack

To deploy 

```
./.deploy/deploy.sh
```

