# CRISP SDK

TypeScript SDK for interacting with CRISP (Cryptographically Secure and Private voting protocol) and
the CRISP server.

## Installation

```bash
npm install @crisp-e3/sdk
```

## Features

- **Round Management**: Fetch round details, token requirements, and voting parameters
- **Token Operations**: Query token balances and total supply at specific blocks
- **Merkle Tree Utilities**: Generate proofs for voter inclusion in the eligibility tree
- **Vote Proof Generation**: Create zero-knowledge proofs for votes and mask votes
- **Proof Verification**: Verify generated proofs using Noir circuits

## Usage

### Get Round Details

```typescript
import { getRoundDetails } from '@crisp-e3/sdk'

const roundDetails = await getRoundDetails(serverUrl, e3Id)
```

### Get Token Balance

```typescript
import { getBalanceAt } from '@crisp-e3/sdk'

const balance = await getBalanceAt(voterAddress, tokenAddress, snapshotBlock, chainId)
```

### Generate Vote Proof

```typescript
import { generateVoteProof } from '@crisp-e3/sdk'

const proof = await generateVoteProof({
  vote: { yes: 100n, no: 0n },
  publicKey: publicKeyBytes,
  signature: '0x...',
  balance: 1000n,
  merkleLeaves: [...],
})
```

### Generate Mask Vote Proof

```typescript
import { generateMaskVoteProof } from '@crisp-e3/sdk'

const maskProof = await generateMaskVoteProof({
  balance: 1000n,
  slotAddress: '0x...',
  publicKey: publicKeyBytes,
  merkleLeaves: [...],
  previousCiphertext: previousCiphertextBytes, // optional
})
```

### Verify Proof

```typescript
import { verifyProof } from '@crisp-e3/sdk'

const isValid = await verifyProof(proof)
```

## API

- **State**: `getRoundDetails`, `getRoundTokenDetails`
- **Token**: `getBalanceAt`, `getTotalSupplyAt`, `getTreeData`
- **Vote**: `generateVoteProof`, `generateMaskVoteProof`, `verifyProof`, `decodeTally`
- **Utils**: `generateMerkleProof`, `generateMerkleTree`, `hashLeaf`, `getAddressFromSignature`
