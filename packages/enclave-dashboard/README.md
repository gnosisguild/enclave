# @enclave-e3/dashboard

Interfold / CRISP public observation dashboard. Two tabs:

- **CRISP** — hero poll card, live 7-stage timeline, expandable history, network pulse footer.
  Observational only (no vote CTA).
- **E3 inspector** — deep technical record of one E3: request, committee, keygen rounds, input
  window, compute, decryption, publication, fees, on-chain event log.

## Run

```bash
pnpm install
pnpm --filter @enclave-e3/dashboard dev
```

Opens at `http://localhost:5173`.

## On-chain backend (Sepolia)

The dashboard reads live data from the Sepolia deployment of Enclave
(`packages/enclave-contracts/deployed_contracts.json`). ABIs come from the canonical typechain
factories in `@enclave-e3/contracts/types` and the `E3Stage`/`FailureReason` enums from
`@enclave-e3/sdk/contracts` so the dashboard cannot drift from the deployed contracts.

- `Enclave` proxy at `0xB47B267876B60a06138Bc9dfCee7aa3E26907CCB` — `E3Requested`, `InputPublished`,
  `PlaintextOutputPublished`, plus `getE3` / `getE3Stage` view functions.
- `CiphernodeRegistryOwnable` at `0x497Feea9abB72229aab1584c22b5416ff128926B` — `CommitteeRequested`
  (threshold), `CommitteeFinalized` (members), `CommitteePublished` (joint PK).
- `CRISPProgram` at `0xba3B07aBFd0B8cad68aa1E946CC7AF5C1B1c8B5D` — the program address shown in the
  inspector.

CRISP question text + option labels are off-chain (the program doesn't store them); the mapping
lives in `src/lib/pollMeta.ts`. Unknown E3 ids get a generic "Encrypted poll #N" header with numeric
option labels.

### RPC

Defaults to `https://ethereum-sepolia.publicnode.com`. Override via:

```
VITE_SEPOLIA_RPC=https://eth-sepolia.g.alchemy.com/v2/<key>
```

The fetchers chunk `getLogs` calls to 9_500 blocks per request so they work against the stricter
free-tier providers.

### Polling

`useE3List` and `useE3Details` poll every 15 seconds while mounted. When the chain-derived stage
advances, the CRISP tab's stage + pollState reconcile automatically; manual overrides via the Tweaks
panel still work (they're clobbered on the next poll tick).

## Build

```bash
pnpm --filter @enclave-e3/dashboard build       # vite build → dist/
pnpm --filter @enclave-e3/dashboard typecheck   # tsc --noEmit
pnpm --filter @enclave-e3/dashboard preview     # serve dist/
```
