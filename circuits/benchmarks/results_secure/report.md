# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-27 12:41:40 UTC

**Git Branch:** `fix/configs-circuit`  
**Git Commit:** `eca330d9a59f5433c443a2880bc82aeedac8681b`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 287764      | 1.60           | 26.29            | 15.88           |
| C1                   | 2432074     | 10.37          | 30.21            | 15.88           |
| C2a                  | 3879330     | 10.93          | 24.41            | 15.88           |
| C2b                  | 5739750     | 19.84          | 25.13            | 15.88           |
| C3a                  | 3764144     | 12.67          | 31.34            | 15.88           |
| C3b                  | 3764144     | 12.67          | 31.34            | 15.88           |
| C4a                  | 2564001     | 10.29          | 30.31            | 15.88           |
| C4b                  | 2564001     | 10.29          | 30.31            | 15.88           |
| C5                   | 4395328     | 19.30          | 29.97            | 15.88           |
| user_data_encryption | 1678200     | 6.34           | 30.13            | 15.88           |
| C6                   | 3001847     | 10.46          | 26.15            | 15.88           |
| C7                   | 128310      | 0.53           | 25.75            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 15.88 KB   | 0.12 KB           | 3037824    | 202192       | 3240016   |
| Π_user   | 31.75 KB   | 0.22 KB           | 2972929    | 386136       | 3359065   |
| Π_dec    | 15.88 KB   | 3.25 KB           | 3549028    | 188124       | 3737152   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 65.70 s    | 95.25 KB   | 96.41 KB  |
| Aggregator      | P2    | combine folds + C5               | 19.30 s    | 15.88 KB   | 16.00 KB  |
| User            | P3    | per user input                   | 12.14 s    | 31.75 KB   | 31.97 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 10.46 s    | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 0.53 s     | 15.88 KB   | 19.12 KB  |

## System Information

### Hardware

- **CPU:** Apple M4 Pro
- **CPU Cores:** 14
- **RAM:** 48.00 GB
- **OS:** Darwin
- **Architecture:** arm64

### Software

- **Nargo Version:** nargo version = 1.0.0-beta.16 noirc version =
  1.0.0-beta.16+2d46fca7203545cbbfb31a0d0328de6c10a8db95 (git version hash:
  2d46fca7203545cbbfb31a0d0328de6c10a8db95, is dirty: false)
- **Barretenberg Version:** 3.0.0-nightly.20260102
