# CRISP

## Prerequissites

- Docker (v25.0.6)

## Setup

```
./setup.sh
```

## Develop

```
./dev.sh
```

Then you should be able view the client here:

```
http://localhost:3000
```

### Anvil doesn't close

The terminal gets stuck with anvil and to close the terminal you can try:

```
docker compose exec enclave-dev pkill -9 -f anvil
```
