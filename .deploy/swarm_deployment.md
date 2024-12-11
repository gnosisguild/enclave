
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

# Run docker swarm

First we need to initialize swarm.

```
docker swarm init
```

If you get an error about not being able to choose between IP addresses choose the more private IP address.

```
docker swarm init --advertise-addr 10.49.x.x
```

```
TAG=latest docker stack deploy -c .deploy/docker-compose.yml enclave-stack --detach=false
```

