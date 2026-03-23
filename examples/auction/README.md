# Sealed-Bid Auction Demo (FHE)

Demonstrates homomorphic bid comparison using BFV SIMD encoding. Bids are
encrypted client-side, compared homomorphically on the server, and only the
winner is revealed.

## Quick Start

### Standalone demo (no server)

```bash
cd examples/auction
cargo run
```

### Server + Client

**Terminal 1 — auction server:**

```bash
cd examples/auction/server
RUST_LOG=info cargo run
```

**Terminal 2 — React client:**

```bash
cd examples/auction/client
npm install
npm run dev
```

Open <http://localhost:5173>.

### WASM client-side encryption (optional)

Build the WASM module for true client-side bid encryption:

```bash
cd examples/auction/wasm
wasm-pack build --target web
```

## API

| Endpoint | Method | Description |
|---|---|---|
| `/health` | GET | Health check |
| `/auction/create` | POST | Create a new auction, returns `{ id, public_key }` |
| `/auction/{id}` | GET | Get auction state |
| `/auction/{id}/bid` | POST | Submit encrypted bid `{ address, ciphertext }` |
| `/auction/{id}/encrypt` | POST | Server-side encryption helper `{ bid }` → `{ ciphertext }` |
| `/auction/{id}/close` | POST | Close auction, run FHE comparison, return winner |
| `/auction/{id}/result` | GET | Get winner |

Ciphertext and public key values are base64-encoded.

## How It Works

1. **SIMD binary encoding** — each bid is encoded as 10 binary bits across
   SIMD slots of a single BFV ciphertext (MSB first).
2. **Prefix-scan comparison** — a homomorphic circuit computes per-bit
   greater-than and equality signals, then merges them via a parallel prefix
   scan using column rotations.
3. **Tournament** — pairwise comparisons determine the overall winner.
   Only the winning bid is decrypted.

Parameters: N=2048, t=12289, 6×62-bit moduli. Bid range: 0–1023.
